# ER Diagram

This diagram shows table relationships. Discord IDs (`guild_id`, `*_channel_id`, `host_user_id`, `task_id`) are stored as `BIGINT` and contain Discord snowflake IDs directly.

## Full Diagram

```mermaid
erDiagram
    guilds ||--o{ guild_channels : has
    guilds ||--o{ rooms : has
    guilds ||--o{ rental_question_presets : has
    guilds ||--o{ rental_sessions : has
    rental_question_presets ||--o{ rooms : "preset (SET NULL)"
    rooms ||--o{ rental_sessions : "rented as"
    rental_sessions ||--o{ scheduled_tasks : "schedules"
    scheduled_tasks ||--o{ notifications : "fires"

    guilds {
        BIGINT      guild_id PK "Discord guild id"
        VARCHAR(5)  language  "ja / en (default ja)"
        TIMESTAMPTZ created_at
        TIMESTAMPTZ updated_at
    }

    guild_channels {
        SERIAL      id PK
        BIGINT      guild_id FK
        BIGINT      channel_id "Discord channel id"
        SMALLINT    channel_type "1=report, 2=rental_button"
        TIMESTAMPTZ created_at
    }

    rooms {
        SERIAL      id PK
        BIGINT      guild_id FK
        BIGINT      text_channel_id "nullable"
        BIGINT      voice_channel_id "nullable"
        BOOLEAN     is_available "default TRUE"
        INT         question_preset_id FK "nullable"
        TIMESTAMPTZ created_at
    }

    rental_question_presets {
        SERIAL      id PK
        BIGINT      guild_id FK
        TEXT        name "unique within guild"
        TEXT        question_1 "nullable"
        TEXT        question_2 "nullable"
        TEXT        question_n "...up to question_10"
        TIMESTAMPTZ created_at
        TIMESTAMPTZ updated_at
    }

    rental_sessions {
        SERIAL      id PK
        BIGINT      guild_id FK
        INT         room_id FK
        BIGINT      host_user_id "Discord user id"
        TEXT        purpose "nullable"
        SMALLINT    state "1=awaiting,2=active,3=released,4=pending_handoff"
        TIMESTAMPTZ started_at
        TIMESTAMPTZ purpose_deadline "nullable"
        TIMESTAMPTZ ended_at "nullable"
    }

    scheduled_tasks {
        SERIAL      id PK
        BIGINT      guild_id "denormalized"
        SMALLINT    task_type "1=timeout_notification"
        INT         rental_session_id FK "nullable"
        TIMESTAMPTZ schedule_datetime
        BOOLEAN     processed "default FALSE"
        TIMESTAMPTZ created_at
    }

    notifications {
        SERIAL      id PK
        INT         task_id FK
        BIGINT      guild_id "denormalized"
        TIMESTAMPTZ schedule_datetime
        BOOLEAN     sent "default FALSE"
        TIMESTAMPTZ sent_at "nullable"
        TIMESTAMPTZ created_at
    }
```

## Reading the Relationships

### Guild Tree (`guild_master` Schema)

`guilds` is the root, and all child tables have `guild_id`.
`guild_channels`, `rooms`, and `rental_question_presets` use `ON DELETE CASCADE`, so deleting a guild deletes these dependent rows. `rental_sessions` has no `ON DELETE` action for `guild_id` (`NO ACTION`), assuming history will be handled separately before deleting a guild.

`rooms` -> `rental_question_presets` uses `ON DELETE SET NULL`: deleting a preset leaves the room and only removes the association.

### Sessions and Scheduler (Bridge to the `worker` Schema)

`scheduled_tasks` references `rental_sessions(id)` with `ON DELETE CASCADE`. Physically deleting a session also deletes its schedule, but operationally sessions are not physically deleted; they are updated to `state=3 (released)`.

`scheduled_tasks` and `notifications` store `guild_id` in a **denormalized** form. Because the `worker` schema does not have RLS, `guild_id` is both a logical separation key and an index key.

### Unique Constraints

| Table | UNIQUE constraint | Meaning |
|---|---|---|
| `guild_channels` | `(guild_id, channel_type)` | One channel per type per guild |
| `rooms` | `(guild_id, text_channel_id)` | The same text channel cannot be registered to two rooms |
| `rooms` | `(guild_id, voice_channel_id)` | The same VC cannot be registered to two rooms |
| `rental_question_presets` | `(guild_id, name)` | Preset names are unique within a guild |

### Optional Field Combinations

Both `text_channel_id` and `voice_channel_id` on `rooms` are nullable. This allows one table to represent three forms: text-only, VC-only, and paired text+VC.
The rule that **at least one must have a value** is enforced by application validation (`AdminRoomAtLeastOne` in `/register_room`).
