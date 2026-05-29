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
| Admin responses | All admin command responses are `ephemeral` |

## Admin Commands

Permission requirement: all admin commands require `ADMINISTRATOR` or `MANAGE_GUILD` permissions (`check_admin_permission`).

| Command | Purpose |
|---|---|
| [`/upload_guild_config`](#upload_guild_config) | Upload the whole-guild YAML configuration |
| [`/download_guild_config`](#download_guild_config) | Download the current whole-guild configuration as YAML |
| [`/list_question_presets`](#list_question_presets) | List registered question presets |
| [`/list_rooms`](#list_rooms) | List registered rooms |

The previous individual `register_*` / `delete_*` / `set_*` admin commands have been removed in favor of full YAML replacement.
See [Server Admins: Guild Config YAML](../server-admins/guild-config-yaml.md) for the schema and operational policy.

## User Commands

| Command | Purpose |
|---|---|
| [`/rent`](#rent) | Start a rental request |
| [`/help`](#help) | Show command help |

---

## `/upload_guild_config`

Applies a whole-guild configuration (channels, question presets, rooms, groups, routing rules) from a single YAML attachment.
The YAML is treated as the source of truth — anything missing from the attachment is deleted (full replacement semantics).

| Item | Details |
|---|---|
| Type | Admin |
| Options | `file: Attachment` (required) — the YAML file |
| Main calls | `reqwest` fetches the attachment URL → `facade::guild_config::parse()` → `facade::guild_config::apply()` inside `with_guild_context()` |
| Transaction | Single transaction on the `db.guild` pool. RLS is enforced via `app.current_guild_id` |
| Affected tables | `guild_channels`, `rental_question_presets`, `room_groups`, `rooms`, `rental_routing_rules` (also optionally updates `guilds.language`) |
| Cascade deletes | `rental_sessions` / `scheduled_tasks` cascade away with the parent `rooms` rows at the DB level |
| Force release | After commit, `rental::routing::force_release_for_rooms` aborts in-memory state and timeout tasks, then DMs hosts |
| Response | `MessageKey::BotConfigUploadSuccess`, plus `BotConfigUploadActiveSessionsReleased` when force release fired |
| Error responses | YAML parse failure → `BotConfigUploadErrorYaml`; validation failure → `BotConfigUploadErrorValidation` (with the validation messages appended); attachment fetch failure → `BotConfigUploadErrorAttachment` |

Validation is completed inside `parse()` and returns *all* discovered errors at once (no fail-fast — every problem is surfaced together).

## `/download_guild_config`

Dumps the current DB state to YAML and returns it as a Discord attachment.
The bot keeps no history; the response is always a fresh snapshot.

| Item | Details |
|---|---|
| Type | Admin |
| Options | None |
| Main calls | `facade::guild_config::dump()` inside `with_guild_context()` |
| Response | An `attachment` named `guild_config.yaml` |
| Empty case | When nothing is configured the bot still returns a YAML body with `version: 1` so clients can consume it uniformly |

The dump output is guaranteed to be re-parseable by `parse()` — operators can edit and re-upload the same file (round-trip safe).

## `/list_question_presets`

Returns a textual listing of registered question presets.

| Item | Details |
|---|---|
| Type | Admin |
| Options | None |
| Main calls | `facade::question_preset::list_presets()` |
| Response | 0 entries → `AdminQuestionPresetListEmpty`; >0 entries → `AdminQuestionPresetListHeader` + a list of preset names |

## `/list_rooms`

Returns a textual listing of registered rooms together with their VC / TC IDs and preset names.

| Item | Details |
|---|---|
| Type | Admin |
| Options | None |
| Main calls | `facade::room::list_rooms()` + preset-name resolution |
| Response | 0 entries → `AdminRoomListEmpty`; >0 entries → `AdminRoomListHeader` + one `AdminRoomListItem` line per room |

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

After a successful modal submission, `rental::routing::post_purpose()` runs as a hook: it looks up `rental_routing_rules` using the answer to the question pointed to by the preset's `routing_key_index`, renders the matching template, and posts to the matching channel (falling back to the configured fallback channel). Posting failures are logged via `tracing::warn!` and never block the rental flow.

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

- [event-specification.md](event-specification.md) — non-command triggers (VC joins, buttons, modal submissions, the routing post hook)
- [configuration.md](configuration.md) — environment variables and DB tables affected by YAML
- [Server Admins: Getting Started](../server-admins/getting-started.md) — operational setup steps
- [Server Admins: Guild Config YAML](../server-admins/guild-config-yaml.md) — full YAML schema
- [Server Admins: Command Reference](../server-admins/command-reference.md) — non-technical explanation for users and admins
