# Command Specification

This document describes the slash commands registered by otachidai Bot.
Command definitions live in `src/discord/commands/register.rs`, and handlers live under `src/discord/commands/{admin,user}/`.
Routing is performed by `events::interaction::handle_command`.

## Common Behavior

| Item | Details |
|---|---|
| Scope | Global registration (`set_global_commands`). Every guild receives the same commands |
| Available locations | Guilds only. Calls from DMs are ignored because `guild_id` is `None` |
| Language | Resolved with `language::resolve_language(state, guild_id, interaction.locale)` and used for i18n responses |
| Permission denial response | Returns `MessageKey::AdminPermissionDenied` as an ephemeral message |
| Handler failure response | Logs with `tracing::error!` and returns the string `"Error"` |

## Admin Commands

Permission requirement: all admin commands require `ADMINISTRATOR` or `MANAGE_GUILD` permissions (`check_admin_permission`).

| Command | Purpose |
|---|---|
| [`/register_report_channel`](#register_report_channel) | Register the text channel that receives reports |
| [`/register_rental_button_channel`](#register_rental_button_channel) | Register the channel where the rental button is posted |
| [`/register_question_preset`](#register_question_preset) | Create or update a question preset for rental purposes |
| [`/register_room`](#register_room) | Register a rentable room |
| [`/delete_room`](#delete_room) | Delete a registered room |

## User Commands

| Command | Purpose |
|---|---|
| [`/rent`](#rent) | Start a rental request |
| [`/help`](#help) | Show command help |

---

## `/register_report_channel`

Registers the notification channel used when a user times out while entering a purpose.

| Item | Details |
|---|---|
| Type | Admin |
| Options | `channel: Channel` (required) — notification destination text channel |
| Side effects | Upserts into `guild_master.guild_channels` with `channel_type=1` |
| Response | `MessageKey::AdminReportChannelRegistered` (i18n, fills `{ $channel }`) |

When specifying a private channel, the bot must separately have **View Channel** and **Send Messages** permissions for that channel. The command itself does not check these permissions.

## `/register_rental_button_channel`

Registers the permanent destination for the rental request button.

| Item | Details |
|---|---|
| Type | Admin |
| Options | `channel: Channel` (required) — text channel where the button is posted |
| Side effects | Upserts into `guild_master.guild_channels` with `channel_type=2` and posts one rental button (`custom_id=rental_start`) to the specified channel |
| Response | `MessageKey::AdminRentalButtonRegistered` (i18n) |

The button is posted every time the command is run, so repeated execution creates additional messages.

## `/register_question_preset`

Registers or updates a question preset shown to users during rental.

| Item | Details |
|---|---|
| Type | Admin |
| Options | `name: String` (required) / `question_1` through `question_10: String` (optional) |
| Validation | Missing `name` -> `AdminQuestionPresetNameRequired` / all of `question_1` through `question_10` empty -> `AdminQuestionPresetAtLeastOne` |
| Side effects | Upserts into `guild_master.rental_question_presets` by `(guild_id, name)`; same name overwrites |
| Response | `MessageKey::AdminQuestionPresetSaved` |

`question_*` provides slots 1 through 10. Empty values and whitespace-only values are skipped (`question_preset::normalize_optional_text`).
Presets are attached to rooms through the `question_preset` argument of `/register_room`.

## `/register_room`

Registers a rentable room. A room can be a text channel only, a voice channel only, or a pair of both.

| Item | Details |
|---|---|
| Type | Admin |
| Options | `text_channel: Channel` (optional) / `voice_channel: Channel` (optional) / `question_preset: String` (optional) |
| Validation | Neither `text_channel` nor `voice_channel` specified -> `AdminRoomAtLeastOne` / specified `question_preset` not found -> `AdminQuestionPresetNotFound` |
| Side effects | Inserts one row into `guild_master.rooms` (`is_available=true`, setting `question_preset_id` when needed) |
| Response | `MessageKey::AdminRoomRegistered` |

Run the command repeatedly to register multiple rooms. There are no `UNIQUE` constraints on the exact channel combination, so be careful not to register the same channel ID combination twice by mistake.

## `/delete_room`

Deletes one registered room.

| Item | Details |
|---|---|
| Type | Admin |
| Options | `text_channel: Channel` (optional) / `voice_channel: Channel` (optional) |
| Behavior | Selects one room by the specified channel ID and deletes it if found |
| Response | Success -> `MessageKey::AdminRoomDeleted` / not found -> `MessageKey::AdminRoomNotFound` |

If neither option is specified, the first room in the guild may become the deletion target, so callers should always provide an argument that identifies the intended room.

---

## `/rent`

Starts a rental request for a user.

| Item | Details |
|---|---|
| Type | User |
| Options | None |
| Behavior | Calls `rental::flow::start_rental(state, guild_id, user_id, voice_channel_id=None, lang)` |
| Response | Branches based on the return value from `start_rental` |

### Response Branches

| Return value | Response |
|---|---|
| `AwaitingQuestions` | Returns the purpose input modal (`InteractionResponseType::Modal`) |
| `Assigned` | Returns an assignment completion message as ephemeral (`MessageKey::BotRentalAssigned`) |
| `AlreadyRenting` | Returns `MessageKey::BotRentalAlreadyRenting` as ephemeral |
| `NoAvailableRooms` | Returns `MessageKey::BotRentalNoRooms` as ephemeral |

Unlike the VC join trigger, this command does not pass `voice_channel_id`, so it assigns the first available room in order (`facade::room::find_available_room`).

## `/help`

Shows bot command help.

| Item | Details |
|---|---|
| Type | User |
| Options | None |
| Response | Concatenates `MessageKey::HelpTitle`, `HelpUser`, and `HelpAdmin` |

There is no permission control here, so user-facing help also includes admin command names. This matches the operational and public documentation policy.

---

## Related Documentation

- [event-specification.md](event-specification.md) — non-command triggers such as VC joins, buttons, and modal submissions
- [configuration.md](configuration.md) — environment variables and DB tables modified by commands
- [Server Admins: Getting Started](../server-admins/getting-started.md) — operational setup steps
- [Server Admins: Command Reference](../server-admins/command-reference.md) — non-technical explanation for users and admins
