# Command Reference (Server Admins)

Commands that require administrator permissions. For user-facing commands, see [Server Users: Command Reference](../server-users/command-reference.md).

Guild configuration (channels, question presets, rooms, routing) is managed via a single YAML file. The individual `register_*` / `delete_*` / `set_*` commands have been retired — all additions, changes, and deletions go through `/upload_guild_config`.

---

## `/upload_guild_config`

Uploads the whole-guild configuration as a YAML file. **The uploaded YAML completely replaces the existing configuration.**

| Item | Details |
|---|---|
| Permission | Server Administrator |
| Argument | `file` … YAML attachment (required) |

**Key behaviour**

- The YAML is parsed → validated → written to the DB in that order. If any step errors, the DB is left untouched.
- Any room, group, preset, routing rule, or channel that does not appear in the uploaded YAML is deleted.
- If a room being deleted has an active rental session, **that session is force-released and its host is notified**.
- If no routing rule matches and no fallback channel is configured, nothing is posted (a warning is logged).

For the YAML schema, see [Guild configuration YAML spec](guild-config-yaml.md).

---

## `/download_guild_config`

Downloads the current whole-guild configuration as a YAML file. The response is an ephemeral attachment visible only to you.

| Item | Details |
|---|---|
| Permission | Server Administrator |
| Argument | None |

To edit your configuration, fetch the current state with this command, edit it locally, then re-upload with `/upload_guild_config`.

---

## `/list_question_presets`

Lists the registered question presets (lightweight state check).

| Item | Details |
|---|---|
| Permission | Server Administrator |
| Argument | None |

---

## `/list_rooms`

Lists the registered rooms (lightweight state check).

| Item | Details |
|---|---|
| Permission | Server Administrator |
| Argument | None |
