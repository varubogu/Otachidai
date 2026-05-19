# Database Migrations

otachidai Bot manages its schema with SeaORM Migration. Migration files live in `migration/src/` and are executed using the `admin` DB role.

## Creating a New Migration

### 1. Add a file

Add a file under `migration/src/`. Naming convention:

```
m<YYYYMMDD>_<6-digit-sequence>_<snake_case_description>.rs
```

Example: `m20240002_000001_add_foo_column.rs`

```rust
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20240002_000001_add_foo_column"  // must match the filename without extension
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // write DDL here
        manager
            .alter_table(/* ... */)
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // rollback logic
        Ok(())
    }
}
```

### 2. Register in `migration/src/lib.rs`

Add the module to the vec returned by `MigratorTrait::migrations()`:

```rust
mod m20240002_000001_add_foo_column;
// ...

vec![
    // existing entries
    Box::new(m20240002_000001_add_foo_column::Migration),
]
```

## Running Migrations

### Development

Verify that the `ADMIN_DB_*` connection variables in `.env.app` are correct before running.

```bash
# Apply all pending migrations
cargo run --manifest-path migration/Cargo.toml -- up

# Apply one migration
cargo run --manifest-path migration/Cargo.toml -- up -n 1

# Roll back one migration
cargo run --manifest-path migration/Cargo.toml -- down -n 1

# Show applied migrations
cargo run --manifest-path migration/Cargo.toml -- status
```

### Docker (production / staging)

```bash
# Apply all pending migrations
docker compose --profile migration run --rm migration up

# Apply one migration
docker compose --profile migration run --rm migration up -n 1

# Roll back one migration
docker compose --profile migration run --rm migration down -n 1

# Show applied migrations
docker compose --profile migration run --rm migration status
```

> **Note**: Migrations run as the `admin` DB role (`ADMIN_DB_USER` / `ADMIN_DB_PASSWORD`). In production, make sure the correct credentials are provided via Docker Compose environment variables or secrets.

## Related Documentation

- [Database Design](database/README.md) — schema layout and role design
- [Roles and RLS](database/roles-and-rls.md) — `admin` role permission scope
- [Configuration](configuration.md) — `ADMIN_DB_*` environment variable reference
