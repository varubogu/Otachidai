# ロールとRLS

お立ち台BotのDB側アクセス制御は2層構造です。

1. **PostgreSQLロール**による粒度の粗い分離（読み書きできるテーブル・操作）
2. **Row Level Security (RLS)**による行単位の分離（ギルド境界の強制）

アプリケーションコードでギルドIDを `WHERE` 句に追加し忘れる事故を防ぐため、**ギルド単位の処理はDB層で必ず分離される**ように設計されています。

## ロール一覧

5つのPostgreSQLロールを使い分けます。ロール作成は `db/sql/init.sql`、テーブル権限の付与は `migration/src/m20240001_000008_grant_permissions.rs` で行います。

| ロール | RLS | CREATEDB | `guild_master` 権限 | `worker` 権限 | 主用途 |
|---|---|---|---|---|---|
| `otachidai_bot_system` | BYPASS | — | SELECT/INSERT/UPDATE/DELETE | SELECT/INSERT/UPDATE/DELETE | スケジューラ／タイマー復元 |
| `otachidai_bot_guild` | 適用 | — | SELECT/INSERT/UPDATE/DELETE（RLS制約付き） | SELECT/INSERT | スラッシュコマンド・VCイベント処理 |
| `otachidai_bot_global` | BYPASS | — | ALL | （付与なし） | マスターデータ更新（外部同期等） |
| `otachidai_bot_admin` | BYPASS | あり | （オーナー） | （オーナー） | マイグレーション、スキーマ変更 |
| `otachidai_bot_cleanup` | BYPASS | — | SELECT/DELETE | SELECT/DELETE | データ削除専用 |

`SEQUENCE` 権限は、書き込みするロール（`system`／`guild`／`global`）にだけ `USAGE` を付与しています。

## アプリ側のロール選択

`AppState` は4つの `DatabaseConnection` プールを抱え、用途に応じて使い分けます（`src/db/connections.rs`）。`cleanup` は別プロセス（クリーンアップジョブ）から接続する想定で、Bot本体のプールには含まれません。

| プール | 環境変数 | 想定接続先ロール |
|---|---|---|
| `state.db.system` | `SYSTEM_DB_*` | `otachidai_bot_system` |
| `state.db.guild` | `GUILD_DB_*` | `otachidai_bot_guild` |
| `state.db.global` | `GLOBAL_DB_*` | `otachidai_bot_global` |
| `state.db.admin` | `ADMIN_DB_*` | `otachidai_bot_admin` |

通常運用で**最も使われるのは `state.db.guild`** で、`with_guild_context()` 経由でトランザクションを張ります。

---

## RLSの設計

### 適用対象

RLS（Row Level Security）は `guild_master` スキーマの**ドメインデータ全テーブル**に適用されています。

| テーブル | RLS | ポリシー名 |
|---|---|---|
| `guild_master.guilds` | ENABLE | `guild_isolation` |
| `guild_master.guild_channels` | ENABLE | `guild_isolation` |
| `guild_master.rooms` | ENABLE | `guild_isolation` |
| `guild_master.rental_sessions` | ENABLE | `guild_isolation` |
| `guild_master.rental_question_presets` | ENABLE | `guild_isolation` |
| `worker.scheduled_tasks` | **DISABLE** | — |
| `worker.notifications` | **DISABLE** | — |

`worker` スキーマには RLS を掛けていません。`system`／`cleanup` ロールが横断的にアクセスする前提で、`guild_id` カラムは論理的な分離キーとして機能します。

### ポリシー定義

すべてのRLSポリシーは同じ構造です（`migration/src/m20240001_000009_enable_rls.rs`、`m20260505_000001_add_rental_question_presets.rs`）。

```sql
CREATE POLICY guild_isolation ON guild_master.<table>
  AS PERMISSIVE
  FOR ALL
  TO otachidai_bot_guild
  USING (guild_id = current_setting('app.current_guild_id', true)::BIGINT);
```

ポイント：

- **`TO otachidai_bot_guild` 限定** — 他ロールはRLSをバイパスするため、ポリシー自体を評価する対象が `guild` ロールのみ
- **`FOR ALL`** — SELECT/INSERT/UPDATE/DELETE すべてに同じ条件
- **`current_setting('app.current_guild_id', true)`** — 第2引数 `true` で「未設定なら NULL を返す」モード。設定し忘れると `guild_id = NULL` となりすべての行がフィルタアウトされる（フェイルクローズ）

### コード側のガード

ポリシーに渡る `app.current_guild_id` は、`db::rls::with_guild_context()` がトランザクション開始直後に `SET LOCAL` で設定します。

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

`SET LOCAL` のスコープはトランザクション内に閉じるため、コネクションプールから返却されたコネクションが別のギルドの設定値を持ち越すことはありません。

### 結果として何が守られるか

| 起こり得るバグ | RLSがないと | RLSあり |
|---|---|---|
| `guild_id` を WHERE 句に書き忘れる | 他ギルドの行が読める／書き換えられる | 0行返る／更新0件 |
| 別ギルドの ID を直接指定する | そのまま操作できる | 0件で扱われる（条件不一致） |
| `app.current_guild_id` を設定し忘れる | （関係なし） | 全行が条件不一致となり 0件 |

アプリ側の単体テストで `guild_id` フィルタが漏れていてもRLSが最終防衛線として機能する、という前提で設計されています。

### RLSのバイパス経路

以下の経路はRLSを通さないため、**コード側で明示的に `guild_id` を WHERE 句や条件に入れる必要**があります。

| 経路 | 用途 | 注意点 |
|---|---|---|
| `state.db.system`（`system` ロール、BYPASSRLS） | スケジューラ復元、`worker` 操作 | `restore_pending_timeouts` のように全ギルド横断で読む処理に限定 |
| `state.db.global`（`global` ロール、BYPASSRLS） | マスターデータ更新 | 現状のコードでは未使用 |
| `state.db.admin`（`admin` ロール、BYPASSRLS） | マイグレーション専用 | 業務ロジックでの利用は禁止 |

`worker` スキーマには RLS が掛かっていないため、`guild` ロールから `scheduled_tasks` を SELECT すると全ギルドの行が見えます。現状コードでは `guild` ロールが `worker` テーブルを参照する処理は `INSERT` のみ（`facade::rental::create_session` のタスク登録）で、SELECT の経路は `system` ロール限定になっています。今後 `guild` ロールから `worker` を参照する処理を追加する場合は、明示的な `guild_id = ?` フィルタを必ず付けてください。

---

## 開発時の注意

### マイグレーション後の権限付与

新規テーブルを追加した場合、`GRANT` と `ALTER TABLE ... ENABLE ROW LEVEL SECURITY`、`CREATE POLICY` をマイグレーション内で明示的に実行する必要があります。
`m20260505_000001_add_rental_question_presets.rs` が一例です。`m20240001_000008_grant_permissions.rs` の `ON ALL TABLES` は**マイグレーション実行時点のテーブルにのみ**適用されるため、後から追加されたテーブルには波及しません。

### テスト環境

統合テスト（`tests/integration/`、`#[ignore]` 付き）は実DBに接続して動かす想定です。テスト時に複数ギルドのデータを混ぜて検証する場合、`with_guild_context()` を必ず通すことでRLSが期待通りに動作することも併せて確認できます。

### 緊急時のRLSバイパス

オペレーションの都合でギルド横断のクエリが必要になった場合は、`admin` ロールで `psql` 接続して直接 SQL を流すのが最短です。Bot プロセス経由で恒常的にバイパスする経路は意図的に用意していません。

## 関連ドキュメント

- [テーブル一覧.md](テーブル一覧.md) — 各テーブルのカラム定義
- [ER図.md](ER図.md) — テーブル間リレーション
- [設定ファイル仕様.md](../設定ファイル仕様.md) — 各DBロールに対応する環境変数
- [アーキテクチャ.md](../アーキテクチャ.md) — DBロールの使い分けの全体像
