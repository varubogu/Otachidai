# Roles and RLS

otachidai Bot's database access control has two layers.

1. Coarse separation by **PostgreSQL roles** (which tables and operations are allowed)
2. Row-level separation by **Row Level Security (RLS)** (enforcing guild boundaries)

To prevent bugs where application code forgets to add `guild_id` to a `WHERE` clause, **guild-scoped processing is always separated at the DB layer**.

## Roles

The system uses five PostgreSQL roles. Role creation is in `db/sql/init.sql`, and table permission grants are in `migration/src/m20240001_000008_grant_permissions.rs`.

| Role | RLS | CREATEDB | `guild_master` permissions | `worker` permissions | Main purpose |
|---|---|---|---|---|---|
| `otachidai_bot_system` | BYPASS | — | SELECT/INSERT/UPDATE/DELETE | SELECT/INSERT/UPDATE/DELETE | Scheduler and timeout restoration |
| `otachidai_bot_guild` | Applied | — | SELECT/INSERT/UPDATE/DELETE (with RLS constraints) | SELECT/INSERT | Slash command and VC event processing |
| `otachidai_bot_global` | BYPASS | — | ALL | None granted | Master data updates, such as external sync |
| `otachidai_bot_admin` | BYPASS | Yes | Owner | Owner | Migrations and schema changes |
| `otachidai_bot_cleanup` | BYPASS | — | SELECT/DELETE | SELECT/DELETE | Data deletion only |

`SEQUENCE` `USAGE` is granted only to roles that write data (`system`, `guild`, and `global`).

## Role Selection in the App

`AppState` holds four `DatabaseConnection` pools and selects them by purpose (`src/db/connections.rs`). `cleanup` is intended for a separate cleanup job process and is not included in the bot's pools.

| Pool | Environment variables | Expected DB role |
|---|---|---|
| `state.db.system` | `SYSTEM_DB_*` | `otachidai_bot_system` |
| `state.db.guild` | `GUILD_DB_*` | `otachidai_bot_guild` |
| `state.db.global` | `GLOBAL_DB_*` | `otachidai_bot_global` |
| `state.db.admin` | `ADMIN_DB_*` | `otachidai_bot_admin` |

The most commonly used pool in normal operation is **`state.db.guild`**, wrapped in a transaction through `with_guild_context()`.

---

## RLS Design

### Applied Tables

RLS is applied to **all domain data tables** in the `guild_master` schema.

| Table | RLS | Policy name |
|---|---|---|
| `guild_master.guilds` | ENABLE | `guild_isolation` |
| `guild_master.guild_channels` | ENABLE | `guild_isolation` |
| `guild_master.rooms` | ENABLE | `guild_isolation` |
| `guild_master.rental_sessions` | ENABLE | `guild_isolation` |
| `guild_master.rental_question_presets` | ENABLE | `guild_isolation` |
| `worker.scheduled_tasks` | **DISABLE** | — |
| `worker.notifications` | **DISABLE** | — |

RLS is not applied to the `worker` schema. `system` and `cleanup` roles are expected to access it across guilds, while the `guild_id` column acts as a logical separation key.

### Policy Definition

All RLS policies share the same structure (`migration/src/m20240001_000009_enable_rls.rs`, `m20260505_000001_add_rental_question_presets.rs`).

```sql
CREATE POLICY guild_isolation ON guild_master.<table>
  AS PERMISSIVE
  FOR ALL
  TO otachidai_bot_guild
  USING (guild_id = current_setting('app.current_guild_id', true)::BIGINT);
```

Important points:

- **Limited to `TO otachidai_bot_guild`** — other roles bypass RLS, so only the `guild` role evaluates the policy
- **`FOR ALL`** — the same condition applies to SELECT/INSERT/UPDATE/DELETE
- **`current_setting('app.current_guild_id', true)`** — the second argument `true` means "return NULL if unset". If setting is forgotten, `guild_id = NULL` filters out every row (fail closed)

### Code-Side Guard

`db::rls::with_guild_context()` sets `app.current_guild_id` with `SET LOCAL` immediately after starting a transaction.

```rust
pub async fn with_guild_context<F, T>(db: &DatabaseConnection, guild_id: u64, f: F) -> BotResult<T>
where
    F: for<'c> FnOnce(&'c DatabaseTransaction) -> BoxFuture<'c, BotResult<T>>,
{
    let txn = db.begin().await?;
    txn.execute_unprepared(&format!("SET LOCAL app.current_guild_id = '{guild_id}'"))
        .await?;
    let result = f(&txn).await?;
    txn.commit().await?;
    Ok(result)
}
```

Because `SET LOCAL` is scoped to the transaction, a connection returned to the pool cannot carry one guild's setting into another guild's operation.

### What This Protects

| Possible bug | Without RLS | With RLS |
|---|---|---|
| Forgetting `guild_id` in a WHERE clause | Rows from other guilds can be read or modified | 0 rows returned / 0 rows updated |
| Directly specifying an ID from another guild | Operation succeeds | Treated as not found because the condition does not match |
| Forgetting to set `app.current_guild_id` | Not relevant | Every row fails the condition, so 0 rows are visible |

The design assumes that RLS is the last line of defense even if unit tests miss a missing `guild_id` filter.

### RLS Bypass Paths

The following paths do not go through RLS, so code must explicitly include `guild_id` in `WHERE` clauses or conditions.

| Path | Purpose | Notes |
|---|---|---|
| `state.db.system` (`system` role, BYPASSRLS) | Scheduler restoration and `worker` operations | Limit to cross-guild reads such as `restore_pending_timeouts` |
| `state.db.global` (`global` role, BYPASSRLS) | Master data updates | Currently unused in application code |
| `state.db.admin` (`admin` role, BYPASSRLS) | Migrations only | Must not be used for business logic |

The `worker` schema has no RLS, so if the `guild` role SELECTs from `scheduled_tasks`, it can see rows from all guilds. Current code only inserts into `worker` through the `guild` role (`facade::rental::create_session` task registration); SELECT paths are limited to the `system` role. If future code reads `worker` tables through the `guild` role, it must include an explicit `guild_id = ?` filter.

---

## Development Notes

### Permission Grants After Migrations

When adding a new table, the migration must explicitly run `GRANT`, `ALTER TABLE ... ENABLE ROW LEVEL SECURITY`, and `CREATE POLICY`.
`m20260505_000001_add_rental_question_presets.rs` is an example. The `ON ALL TABLES` in `m20240001_000008_grant_permissions.rs` applies only to tables that exist at migration execution time, so it does not affect tables added later.

### Test Environment

Integration tests under `tests/integration/` are marked `#[ignore]` and are intended to run against a real DB. When testing data from multiple guilds, always go through `with_guild_context()` so RLS behavior is verified as well.

### Emergency RLS Bypass

If an operation requires a cross-guild query, the shortest path is connecting with `psql` as the `admin` role and running SQL directly. The bot process intentionally does not provide a permanent business-logic path that bypasses RLS.

## Related Documentation

- [tables.md](tables.md) — column definitions for each table
- [er-diagram.md](er-diagram.md) — table relationships
- [../configuration.md](../configuration.md) — environment variables for each DB role
- [../architecture.md](../architecture.md) — overall DB role usage
