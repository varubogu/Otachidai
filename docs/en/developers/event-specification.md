# Event Specification

This document lists the **Gateway events** received from Discord and the **internal events** originating from timers and schedulers that otachidai Bot reacts to.
The entry point is the `dispatch` function in `src/discord/events/dispatcher.rs`, which routes events to sub-handlers.

## Received Gateway Events

`dispatch()` routes the following four event types. Other Gateway events are ignored.

| Event | Handler | Main purpose |
|---|---|---|
| `Ready` | `events::guild::handle_ready` | Detect startup completion and register slash commands globally |
| `GuildCreate` | `events::guild::handle_guild_create` | Detect server joins (currently logs only) |
| `InteractionCreate` | `events::interaction::handle` | Process commands, buttons, and modal submissions |
| `VoiceStateUpdate` | `events::voice_state::handle` | Start, end, and hand off rentals based on VC joins/leaves |

---

### Ready

| Item | Details |
|---|---|
| Trigger | Discord Gateway connection completes |
| Processing | Registers slash commands globally (`commands::register::register_global_commands`) |
| Side effect | Updates the command tree on Discord |

Because commands are registered globally rather than per guild, new commands may take up to about one hour to appear.

### GuildCreate

| Item | Details |
|---|---|
| Trigger | The bot joins a server, or an already joined server is reported during startup |
| Processing | Currently only logs with `tracing::debug!` |
| Side effect | None |

This is a hook where server initialization can be added later.

### InteractionCreate

Interactions outside servers (DMs) are ignored. Any interaction where `guild_id` is `None` returns early.
The handler branches on three subtypes using `interaction.kind`.

#### ApplicationCommand (Slash Command)

| Item | Details |
|---|---|
| Trigger | A user sends a slash command |
| Permission check | For admin command names (`register_*` / `delete_room`), requires `ADMINISTRATOR` or `MANAGE_GUILD` |
| Language resolution | `language::resolve_language(&state, guild_id, interaction.locale)` |
| Routing | Calls `commands::admin::*::handle` or `commands::user::*::handle` based on `data.name` |
| Response | Usually `ChannelMessageWithSource`; only permission denial is ephemeral |

See [command-specification.md](command-specification.md) for command details.

#### MessageComponent (Button Click)

Branches by `custom_id`.

| custom_id | Source | Processing |
|---|---|---|
| `rental_start` | Permanent button in the rental button channel | Runs the same flow as `/rent` (`commands::user::rent::handle`) |
| `rental_start:{session_id}:{room_id}` | Per-session button posted by the bot when a user joins a VC | If the clicking user is the pending host for that session, returns the modal using that room's question preset |
| `handoff_accept:{session_id}:{room_id}` | Handoff button posted when the room host leaves | Returns `DeferredUpdateMessage`, then runs `handoff::accept_handoff` |

For `rental_start:{session_id}:{room_id}`, the bot verifies that the user matches the `host_user_id` of `RentalState::AwaitingPurpose` through `is_pending_rental_host`. If they do not match, it returns an ephemeral error.

#### ModalSubmit

| custom_id | Processing |
|---|---|
| `purpose_modal:{session_id}:{room_id}` | Extracts `purpose_text`, saves the purpose through `rental::flow::submit_purpose`, stops the timer, and transitions to `Active` |

The response is ephemeral and contains the assignment confirmation message.

---

### VoiceStateUpdate

This fires when a user joins, leaves, or moves between VCs.
If `event.channel_id` is `Some`, it is a join or destination of a move; if `None`, it is a leave.
The handler looks up `previous_vc` from the state map and decomposes moves into **leave -> join** before processing.

#### Join (`handle_join`)

| Condition | Behavior |
|---|---|
| Destination VC is not registered in `rooms` | Do nothing |
| State map already has an entry for `(guild_id, vc_id)` | Treat as a rental in progress and skip |
| Otherwise | Start rental (`rental::flow::start_rental`) |

Branches by the return value from `start_rental`:

- `AwaitingQuestions` — posts a message with a purpose modal button (`rental_start:{session_id}:{room_id}`) to the VC's built-in chat or registered text channel. If posting fails, rolls back with `release_rental`
- `Assigned` — occurs for rooms without a question preset. The VC join alone transitions the session to `Active` (no message is posted)
- `AlreadyRenting` / `NoAvailableRooms` — no message is shown for VC triggers (only command paths return ephemeral responses)

Prompt channel resolution uses `prompt_channel_id(text_channel_id, voice_channel_id)`:

- If the room has `text_channel_id`, use that text channel
- Otherwise, use the VC's built-in text chat

#### Leave (`handle_leave`)

The bot branches based on the current state in the `(guild_id, vc_id)` state map and whether the leaving user matches the host.

| Current state match | Action | Behavior |
|---|---|---|
| Host of `AwaitingPurpose` | `CancelPending` | Releases the session with `release_rental` and removes it from the state map (no report is sent) |
| Host of `Active` | `StartHandoff` | Posts a handoff confirmation message with `handoff::initiate_handoff` and transitions to `PendingHandoff` |
| Anything else (for example, non-host participant leaving) | `Ignore` | Do nothing |

---

## Internal Events (Timers)

The bot starts two types of timers during rental flows. They are spawned as Tokio tasks, and their `JoinHandle` is held inside `RentalStateEntry`. When the state advances, they are cancelled with `abort_timeout()`.

### Purpose Timeout (Default 10 Minutes)

| Item | Details |
|---|---|
| Spawned by | `rental::timeout::spawn_purpose_timeout` |
| Trigger | `start_rental` creates an `AwaitingPurpose` session |
| Duration | `facade::rental::PURPOSE_TIMEOUT_MINUTES` minutes (default 10) |
| Behavior when fired | Updates session to `released` / marks `scheduled_tasks` as processed / marks the room available / removes the state map entry / notifies the report channel if registered |
| Cancelled when | Purpose is submitted, the host leaves the VC, or the flow transitions to handoff |

The same deadline is also persisted to `scheduled_tasks` when the session is created. On bot restart, `restore_pending_timeouts` respawns unprocessed tasks, preserving the deadline even after in-memory state is lost.

### Handoff Timeout (Fixed 300 Seconds)

| Item | Details |
|---|---|
| Spawned by | `rental::timeout::spawn_handoff_timeout` |
| Trigger | `handoff::initiate_handoff` after the room host leaves an `Active` session |
| Duration | `handoff::HANDOFF_TIMEOUT_SECS` (300 seconds) |
| Behavior when fired | Updates session to `released` / marks the room available / removes the state map entry |
| Cancelled when | Handoff button is clicked (`handoff_accept`) |

Handoff timeouts are not persisted to `scheduled_tasks`; currently they are managed only in memory. After a restart, DB sessions in `pending_handoff` are expected to be handled manually or by future restoration logic.

---

## Error Handling

`dispatch()` receives `BotResult` from sub-handlers. If it is `Err`, it logs with `tracing::error!` and continues processing.
Because the next event is still received even when an individual event handler fails, transient failures do not stop the whole bot.

## Related Documentation

- [basic-design.md](basic-design.md) — what this bot is trying to achieve
- [architecture.md](architecture.md) — module layout and state transition diagram
- [command-specification.md](command-specification.md) — details of each slash command
- [configuration.md](configuration.md) — environment variables, DB settings, and locale files
