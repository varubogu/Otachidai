# Tables

This document describes tables under the `guild_master` and `worker` schemas.
Migrations under `migration/src/` are the canonical source for column definitions, `src/entities/` for ORM models, and `src/facade/` for the query layer.

## `guild_master` Schema

Domain data. RLS is applied. See [roles-and-rls.md](roles-and-rls.md).

### `guild_master.guilds`

Basic settings per guild. Inserted automatically by `facade::guild_settings::ensure_guild` when a new guild first appears.

| Column | Type | NULL | Default | Description |
|---|---|---|---|---|
| `guild_id` | `BIGINT` | NOT NULL | — | PK. Discord guild ID |
| `language` | `VARCHAR(5)` | NOT NULL | `'ja'` | Display language. `ja` or `en` |
| `created_at` | `TIMESTAMPTZ` | NOT NULL | `now()` | Creation timestamp |
| `updated_at` | `TIMESTAMPTZ` | NOT NULL | `now()` | Update timestamp |

| Constraint | Details |
|---|---|
| PRIMARY KEY | `guild_id` |

Update paths: `ensure_guild` (automatic) / no language change command currently exists

---

### `guild_master.guild_channels`

Configured guild channels. The structure is one guild x one type x one channel.

| Column | Type | NULL | Default | Description |
|---|---|---|---|---|
| `id` | `SERIAL` | NOT NULL | — | PK |
| `guild_id` | `BIGINT` | NOT NULL | — | FK -> `guilds.guild_id` (ON DELETE CASCADE) |
| `channel_id` | `BIGINT` | NOT NULL | — | Discord channel ID |
| `channel_type` | `SMALLINT` | NOT NULL | — | Type code (see below) |
| `created_at` | `TIMESTAMPTZ` | NOT NULL | `now()` | Creation timestamp |

| Constraint | Details |
|---|---|
| PRIMARY KEY | `id` |
| FOREIGN KEY | `guild_id` -> `guilds.guild_id` ON DELETE CASCADE |
| UNIQUE | `(guild_id, channel_type)` |

#### `channel_type` Code Values

Constants are defined in `src/entities/guild_channels.rs`.

| Value | Constant | Purpose |
|---|---|---|
| `1` | `CHANNEL_TYPE_REPORT` | Report channel for timeout notifications |
| `2` | `CHANNEL_TYPE_RENTAL_BUTTON` | Permanent rental button channel |

Update paths: `/register_report_channel`, `/register_rental_button_channel`

---

### `guild_master.rooms`

Rentable rooms. A text-only channel, voice-only channel, or text+VC pair is represented in one table.

| Column | Type | NULL | Default | Description |
|---|---|---|---|---|
| `id` | `SERIAL` | NOT NULL | — | PK |
| `guild_id` | `BIGINT` | NOT NULL | — | FK -> `guilds.guild_id` (ON DELETE CASCADE) |
| `text_channel_id` | `BIGINT` | Nullable | — | Linked text channel |
| `voice_channel_id` | `BIGINT` | Nullable | — | Linked voice channel |
| `is_available` | `BOOLEAN` | NOT NULL | `TRUE` | Availability flag |
| `question_preset_id` | `INT` | Nullable | — | FK -> `rental_question_presets.id` (ON DELETE SET NULL) |
| `created_at` | `TIMESTAMPTZ` | NOT NULL | `now()` | Creation timestamp |

| Constraint | Details |
|---|---|
| PRIMARY KEY | `id` |
| FOREIGN KEY | `guild_id` -> `guilds.guild_id` ON DELETE CASCADE |
| FOREIGN KEY | `question_preset_id` -> `rental_question_presets.id` ON DELETE SET NULL |
| UNIQUE | `(guild_id, text_channel_id)` |
| UNIQUE | `(guild_id, voice_channel_id)` |

The application enforces that at least one of `text_channel_id` and `voice_channel_id` is set; there is no migration-level CHECK constraint.

Update paths: `/register_room`, `/delete_room`, `facade::room::set_room_availability` (automatic during rental start/release)

---

### `guild_master.rental_question_presets`

Question presets for purpose input. One row contains 1 to 10 question strings.

| Column | Type | NULL | Default | Description |
|---|---|---|---|---|
| `id` | `SERIAL` | NOT NULL | — | PK |
| `guild_id` | `BIGINT` | NOT NULL | — | FK -> `guilds.guild_id` (ON DELETE CASCADE) |
| `name` | `TEXT` | NOT NULL | — | Preset name, unique within the guild |
| `question_1` through `question_10` | `TEXT` | Nullable | — | Question text. Empty strings and whitespace-only values are treated as invalid |
| `created_at` | `TIMESTAMPTZ` | NOT NULL | `now()` | Creation timestamp |
| `updated_at` | `TIMESTAMPTZ` | NOT NULL | `now()` | Update timestamp |

| Constraint | Details |
|---|---|
| PRIMARY KEY | `id` |
| FOREIGN KEY | `guild_id` -> `guilds.guild_id` ON DELETE CASCADE |
| UNIQUE | `(guild_id, name)` |

`Model::questions()` (`src/entities/rental_question_presets.rs`) extracts only non-empty values and returns them as `Vec<String>`.

Update path: `/register_question_preset` (rerunning with the same name overwrites)

---

### `guild_master.rental_sessions`

Persistent rental session state. One row represents one session, and `state` changes as the flow progresses.

| Column | Type | NULL | Default | Description |
|---|---|---|---|---|
| `id` | `SERIAL` | NOT NULL | — | PK |
| `guild_id` | `BIGINT` | NOT NULL | — | FK -> `guilds.guild_id` (no ON DELETE action) |
| `room_id` | `INT` | NOT NULL | — | FK -> `rooms.id` (ON DELETE CASCADE) |
| `host_user_id` | `BIGINT` | NOT NULL | — | Discord user ID of the room host |
| `purpose` | `TEXT` | Nullable | — | Submitted usage purpose |
| `state` | `SMALLINT` | NOT NULL | `1` | State code (see below) |
| `started_at` | `TIMESTAMPTZ` | NOT NULL | `now()` | Request timestamp |
| `purpose_deadline` | `TIMESTAMPTZ` | Nullable | — | Deadline for purpose input (default +10 minutes) |
| `ended_at` | `TIMESTAMPTZ` | Nullable | — | Release timestamp |

| Constraint | Details |
|---|---|
| PRIMARY KEY | `id` |
| FOREIGN KEY | `guild_id` -> `guilds.guild_id` |
| FOREIGN KEY | `room_id` -> `rooms.id` ON DELETE CASCADE |

#### `state` Code Values

Constants are defined in `src/entities/rental_sessions.rs`. See [../basic-design.md](../basic-design.md) and [../architecture.md](../architecture.md) for state transitions.

| Value | Constant | State |
|---|---|---|
| `1` | `STATE_AWAITING_PURPOSE` | Waiting for purpose input |
| `2` | `STATE_ACTIVE` | Rented / active |
| `3` | `STATE_RELEASED` | Released |
| `4` | `STATE_PENDING_HANDOFF` | Waiting for handoff confirmation |

Update paths: `facade::rental::*` (`create_session`, `set_purpose`, `release_session`, `transfer_host`, `set_pending_handoff`)

---

## `worker` Schema

Working area for timers and notifications. RLS is **not applied**. `system` and `cleanup` roles are expected to access it across guilds, and `guild_id` is stored as a logical separation key.

### `worker.scheduled_tasks`

Persists timers for purpose input timeouts. `restore_pending_timeouts` uses this table to respawn timers when the bot restarts.

| Column | Type | NULL | Default | Description |
|---|---|---|---|---|
| `id` | `SERIAL` | NOT NULL | — | PK |
| `guild_id` | `BIGINT` | NOT NULL | — | Target guild (denormalized) |
| `task_type` | `SMALLINT` | NOT NULL | — | Task type (see below) |
| `rental_session_id` | `INT` | Nullable | — | FK -> `rental_sessions.id` (ON DELETE CASCADE) |
| `schedule_datetime` | `TIMESTAMPTZ` | NOT NULL | — | Fire time |
| `processed` | `BOOLEAN` | NOT NULL | `FALSE` | Completion flag |
| `created_at` | `TIMESTAMPTZ` | NOT NULL | `now()` | Creation timestamp |

| Constraint | Details |
|---|---|
| PRIMARY KEY | `id` |
| FOREIGN KEY | `rental_session_id` -> `rental_sessions.id` ON DELETE CASCADE |

#### `task_type` Code Values

| Value | Constant | Purpose |
|---|---|---|
| `1` | `TASK_TYPE_TIMEOUT_NOTIFICATION` | Purpose input timeout |

Handoff timeouts (5 minutes) are currently managed only in memory and are not persisted here.

Update paths: `facade::rental::create_session` (INSERT) / `mark_task_processed` (UPDATE) / timeout firing sets `processed=true`

---

### `worker.notifications`

Table reserved for storing report-channel send history. **The current code does not read from or write to it**; it already exists through migrations for future use.

| Column | Type | NULL | Default | Description |
|---|---|---|---|---|
| `id` | `SERIAL` | NOT NULL | — | PK |
| `task_id` | `INT` | NOT NULL | — | FK -> `scheduled_tasks.id` (ON DELETE CASCADE) |
| `guild_id` | `BIGINT` | NOT NULL | — | Target guild (denormalized) |
| `schedule_datetime` | `TIMESTAMPTZ` | NOT NULL | — | Scheduled send time |
| `sent` | `BOOLEAN` | NOT NULL | `FALSE` | Sent flag |
| `sent_at` | `TIMESTAMPTZ` | Nullable | — | Actual sent time |
| `created_at` | `TIMESTAMPTZ` | NOT NULL | `now()` | Creation timestamp |

| Constraint | Details |
|---|---|
| PRIMARY KEY | `id` |
| FOREIGN KEY | `task_id` -> `scheduled_tasks.id` ON DELETE CASCADE |

Update paths: none currently. It is intended for future features such as retry management for failed reports.

---

## Naming Conventions

| Type | Convention |
|---|---|
| Primary keys | `id` (`SERIAL`). The only exception is `guilds.guild_id`, which uses the Discord-derived ID itself as the PK |
| Foreign keys | `<table>_id` of the referenced table, for example `room_id` or `question_preset_id` |
| Timestamps | `*_at` (`TIMESTAMPTZ`). The app writes UTC timestamps, assuming `TZ=UTC` |
| Booleans | `is_*` or past participles such as `processed` and `sent` |
| Discord-derived IDs | `BIGINT` (`i64`). `u64` snowflakes are cast with `as i64` before storage |
| Enum-like values | `SMALLINT` plus corresponding `pub const` definitions on the Rust side |

`SMALLINT` is used instead of PostgreSQL `ENUM` to keep SeaORM mapping simple. Adding a code value only requires adding a Rust constant, not a migration.
