//! 結合テストのエントリポイント。
//!
//! ## ローカル実行
//!
//! 結合テストは実 PostgreSQL（ロール/RLS/スキーマを構築済み）を必要とし、すべて
//! `#[ignore]` が付いている。通常の `cargo test` では走らず、`--ignored` 指定で実行する。
//!
//! ```bash
//! docker compose up -d                 # テスト用 PostgreSQL を起動
//! cargo test -- --ignored              # 結合テストを実行
//! ```
//!
//! 接続情報は環境変数（`DB_HOST` / `DB_PORT` / `DB_NAME` / `GUILD_DB_USER` /
//! `GUILD_DB_PASSWORD`）または `.env` から読む。各テストはトランザクション rollback で
//! 分離されるため、テスト同士が干渉せず DB にデータを残さない（[`integration::support`] 参照）。
//! CI では `.github/workflows/ignored-db-tests.yml` がこれらを実行する。

mod integration;
