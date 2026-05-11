# Database Design

This is the design document for otachidai Bot's PostgreSQL schema and the access control built on top of it.
The implementation is split across `src/entities/` (ORM entities), `src/facade/` (domain query helpers), and `migration/src/` (migrations), so this folder focuses on the **schema structure** and **role / row-level access control design**.

## Files in This Folder

| File | Contents |
|---|---|
| [er-diagram.md](er-diagram.md) | Visualizes table relationships with Mermaid |
| [tables.md](tables.md) | Column specs, constraints, and code value mappings for all tables |
| [roles-and-rls.md](roles-and-rls.md) | The five DB roles, RLS policies, and the guard mechanism (`with_guild_context`) |

## Schema Layout

PostgreSQL contains two application schemas.

| Schema | Purpose | Tables |
|---|---|---|
| `guild_master` | Guild-scoped master data and operational state | `guilds`, `guild_channels`, `rooms`, `rental_question_presets`, `rental_sessions` |
| `worker` | Working area for background processing such as timers and notifications | `scheduled_tasks`, `notifications` |

The schemas are separated to draw a clear boundary between **domain data** and **processing queues**. RLS is applied only to `guild_master`; `worker` is intended to be accessed across guilds by scheduler and cleanup roles.

## Role Overview

See [roles-and-rls.md](roles-and-rls.md) for details.

| Role | RLS | Main purpose |
|---|---|---|
| `otachidai_bot_system` | BYPASS | Scheduler and timeout restoration |
| `otachidai_bot_guild` | Applied | Guild-scoped processing such as slash commands and VC events |
| `otachidai_bot_global` | BYPASS | Master data updates, such as external sync |
| `otachidai_bot_admin` | BYPASS | Migrations and schema changes |
| `otachidai_bot_cleanup` | BYPASS | Data deletion only |

## Design Principles

### Guild Isolation at the Schema Layer

In one DB shared by multiple servers, data from one server must not be visible from another server. This is **enforced with RLS rather than application checks**.
All normal operations on `guild_master.*` must go through `db::rls::with_guild_context()`, which sets `SET LOCAL app.current_guild_id` inside the transaction. See [roles-and-rls.md](roles-and-rls.md) for details.

### Dual State in Memory and DB

Rental session state is stored both in `RentalStateMap` (`DashMap`) and in `rental_sessions.state`.
Memory is used for responsiveness; the DB is used for recovery after restarts or failures. `scheduled_tasks` is persisted for the same reason, and startup uses `restore_pending_timeouts` to respawn unprocessed tasks.

### Mutable Status Instead of Immutable History

`rental_sessions` uses one row per session, and state transitions update the `state` column. There is no separate history table.
If audit requirements appear later, update logs for `rental_sessions.state` can be added separately, for example under the `worker` schema.

## Related Documentation

- [../basic-design.md](../basic-design.md) — bot concept and design principles
- [../architecture.md](../architecture.md) — module layout and state transitions
- [../configuration.md](../configuration.md) — DB environment variables and guild-scoped setting update paths
