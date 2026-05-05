データベースマイグレーションを作成・適用します。

## 新しいマイグレーションを作成する

`migration/src/` にファイルを追加します。ファイル名の形式:

```
m<YYYYMMDD>_<6桁連番>_<snake_case説明>.rs
```

例: `m20240002_000001_add_foo_column.rs`

### ファイルテンプレート

```rust
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20240002_000001_add_foo_column"  // ファイル名と一致させる
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // DDLをここに記述
        manager
            .alter_table(/* ... */)
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ロールバック処理
        Ok(())
    }
}
```

### migration/src/lib.rs への登録

`MigratorTrait::migrations()` の vec にモジュールを追加:

```rust
mod m20240002_000001_add_foo_column;
// ...
vec![
    // 既存エントリ
    Box::new(m20240002_000001_add_foo_column::Migration),
]
```

## マイグレーションを実行する

```bash
# 未適用のマイグレーションをすべて適用
cargo run --manifest-path migration/Cargo.toml -- up

# 1件だけ適用
cargo run --manifest-path migration/Cargo.toml -- up -n 1

# 1件ロールバック
cargo run --manifest-path migration/Cargo.toml -- down -n 1

# 適用済み一覧を確認
cargo run --manifest-path migration/Cargo.toml -- status
```

Dockerを使う場合:

```bash
docker compose --profile migration run --rm migration up
```

> **注意**: マイグレーションは `admin` DBロール（`ADMIN_DB_USER`/`ADMIN_DB_PASSWORD`）で実行されます。`.env.app` に正しい値が設定されていることを確認してください。

$ARGUMENTS
