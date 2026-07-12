//! 結合テスト共通ハーネス。
//!
//! 実 PostgreSQL（テスト専用DB）に対して `cargo test -- --ignored` で走らせる。
//!
//! ## テスト間分離
//!
//! 各テストは [`with_rollback_txn`] / [`with_test_guild`] が開くトランザクション内で実行され、
//! 終了時に **必ず rollback** される。これにより:
//! - テスト同士がデータを共有せず、毎回クリーンな状態から始まる
//! - 単一のテスト専用DBを使い回せる（テーブルの TRUNCATE 等の後始末が不要）
//!
//! 本番の [`otachidai::db::rls::with_guild_context`] は commit するが、テストでは
//! 同じ `SET LOCAL app.current_guild_id` を張ったうえで rollback するため、RLS は
//! 本番と同条件で効く。

use futures::future::BoxFuture;
use otachidai::error::BotResult;
use otachidai::facade::{guild_settings, room as room_facade};
use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DatabaseTransaction, TransactionTrait,
};

/// テスト専用DBへの接続文字列を環境変数から組み立てる。
/// 既存の facade テストと同じ変数（`GUILD_DB_*` / `DB_*`）を使う。
fn db_url() -> String {
    let host = std::env::var("DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let user = std::env::var("GUILD_DB_USER").unwrap_or_else(|_| "otachidai_guild".to_string());
    let pass = std::env::var("GUILD_DB_PASSWORD").unwrap_or_default();
    let name = std::env::var("DB_NAME").unwrap_or_else(|_| "otachidai_db".to_string());
    format!("postgres://{user}:{pass}@{host}:{port}/{name}")
}

/// テスト専用DBへ接続する。
pub async fn connect() -> DatabaseConnection {
    dotenvy::dotenv().ok();
    Database::connect(db_url())
        .await
        .expect("テスト専用DBへ接続できること（docker compose で PostgreSQL を起動しているか確認）")
}

/// 開いているトランザクションに RLS のギルドコンテキストを張る。
/// 1トランザクション内で複数回呼べば、ギルドを切り替えられる（RLS分離テストで使用）。
pub async fn set_guild(txn: &DatabaseTransaction, guild_id: u64) {
    txn.execute_unprepared(&format!("SET LOCAL app.current_guild_id = '{guild_id}'"))
        .await
        .expect("ギルドコンテキストを設定できること");
}

/// `f` を「終了時に必ず rollback されるトランザクション」内で実行する。
/// ギルドコンテキストは張らないので、`f` の中で [`set_guild`] を呼ぶこと。
pub async fn with_rollback_txn<F, T>(f: F) -> T
where
    F: for<'c> FnOnce(&'c DatabaseTransaction) -> BoxFuture<'c, BotResult<T>>,
{
    let db = connect().await;
    let txn = db.begin().await.expect("トランザクション開始");
    let result = f(&txn).await.expect("テスト本体が成功すること");
    txn.rollback().await.expect("rollback");
    result
}

/// [`with_rollback_txn`] と同様に rollback されるトランザクションを開き、
/// 先頭で RLS ギルドコンテキストを張る。通常の単一ギルドのテストはこれを使う。
pub async fn with_test_guild<F, T>(guild_id: u64, f: F) -> T
where
    F: for<'c> FnOnce(&'c DatabaseTransaction) -> BoxFuture<'c, BotResult<T>>,
{
    let db = connect().await;
    let txn = db.begin().await.expect("トランザクション開始");
    set_guild(&txn, guild_id).await;
    let result = f(&txn).await.expect("テスト本体が成功すること");
    txn.rollback().await.expect("rollback");
    result
}

/// ギルドを初期化し、VC付きの部屋を1つ登録してその `room_id` を返す。
/// `rental_sessions` は `rooms` / `guilds` への FK を持つため、セッション系テストの前準備に使う。
pub async fn seed_room(
    txn: &DatabaseTransaction,
    guild_id: u64,
    voice_channel_id: u64,
) -> BotResult<i32> {
    guild_settings::ensure_guild(txn, guild_id).await?;
    let room =
        room_facade::register_room(txn, guild_id, None, Some(voice_channel_id), None, None).await?;
    Ok(room.id)
}
