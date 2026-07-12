//! レンタルのライフサイクルを facade の組み合わせで再現する結合テスト。
//!
//! `flow.rs` は Discord `HttpClient`（`AppState`）に依存するため完全実行は手動E2Eの範囲だが、
//! 「DB にどう状態が落ちるか」のシーケンス（申請→目的入力→解放→委譲→タイムアウト相当）は
//! facade レベルでここに集約して検証する。
//!
//! `cargo test -- --ignored` で実行（実 PostgreSQL が必要）。

use crate::integration::support::with_test_guild;
use otachidai::entities::rental_sessions::{STATE_ACTIVE, STATE_PENDING_HANDOFF, STATE_RELEASED};
use otachidai::entities::scheduled_tasks;
use otachidai::facade::{guild_settings, rental, room};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

const HOST: u64 = 11_001;
const NEW_HOST: u64 = 22_002;

/// 申請 → 目的入力 → 解放 までの基本フロー。
#[tokio::test]
#[ignore]
async fn request_then_purpose_then_release() {
    let guild_id: u64 = 999_500_000_000_000_001;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;
            let r = room::register_room(txn, guild_id, None, Some(60_001), None, None).await?;

            // 1. 申請: 空き部屋を取得してセッション作成。
            let available = room::find_available_room(txn, guild_id).await?;
            assert_eq!(available.as_ref().map(|x| x.id), Some(r.id));
            let session = rental::create_session(txn, guild_id, r.id, HOST).await?;
            room::set_room_availability(txn, r.id, false).await?;

            // 申請中は同じ部屋が空きとして返らない。
            assert!(room::find_available_room(txn, guild_id).await?.is_none());

            // 2. 目的入力 → Active。
            let active = rental::set_purpose(txn, session.id, "もくもく作業".to_string()).await?;
            assert_eq!(active.state, STATE_ACTIVE);

            // 3. 解放 → Released、部屋が再び空きに戻る。
            rental::release_session(txn, session.id).await?;
            room::set_room_availability(txn, r.id, true).await?;
            let after = room::find_available_room(txn, guild_id).await?;
            assert_eq!(after.map(|x| x.id), Some(r.id));
            Ok(())
        })
    })
    .await;
}

/// 委譲: pending_handoff → 別ユーザーへ host 移管 → Active。
#[tokio::test]
#[ignore]
async fn handoff_transfers_host() {
    let guild_id: u64 = 999_500_000_000_000_002;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;
            let r = room::register_room(txn, guild_id, None, Some(60_002), None, None).await?;
            let session = rental::create_active_session(txn, guild_id, r.id, HOST).await?;

            rental::set_pending_handoff(txn, session.id).await?;
            let pending = otachidai::entities::rental_sessions::Entity::find_by_id(session.id)
                .one(txn)
                .await?
                .expect("session exists");
            assert_eq!(pending.state, STATE_PENDING_HANDOFF);

            let transferred = rental::transfer_host(txn, session.id, NEW_HOST).await?;
            assert_eq!(transferred.host_user_id, NEW_HOST as i64);
            assert_eq!(transferred.state, STATE_ACTIVE);
            Ok(())
        })
    })
    .await;
}

/// タイムアウト相当: 締切付きセッションを「解放 + タスク処理済み」にする一連の動作。
#[tokio::test]
#[ignore]
async fn timeout_releases_session_and_marks_tasks_processed() {
    let guild_id: u64 = 999_500_000_000_000_003;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;
            let r = room::register_room(txn, guild_id, None, Some(60_003), None, None).await?;
            let session = rental::create_session(txn, guild_id, r.id, HOST).await?;

            // 締切付きのタイムアウトタスクが存在する。
            let task = scheduled_tasks::Entity::find()
                .filter(scheduled_tasks::Column::RentalSessionId.eq(session.id))
                .one(txn)
                .await?
                .expect("timeout task exists");
            assert!(!task.processed);

            // タイムアウト処理: セッションを解放し、紐づくタスクを処理済みにする。
            rental::release_session(txn, session.id).await?;
            rental::mark_session_tasks_processed(txn, session.id).await?;

            let reloaded = otachidai::entities::rental_sessions::Entity::find_by_id(session.id)
                .one(txn)
                .await?
                .expect("session exists");
            assert_eq!(reloaded.state, STATE_RELEASED);

            let after_task = scheduled_tasks::Entity::find_by_id(task.id)
                .one(txn)
                .await?
                .expect("task exists");
            assert!(after_task.processed);
            Ok(())
        })
    })
    .await;
}

/// 同一ユーザーは同時に2つのアクティブなレンタルを持てない（二重申請の検出）。
#[tokio::test]
#[ignore]
async fn user_cannot_hold_two_active_rentals() {
    let guild_id: u64 = 999_500_000_000_000_004;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;
            let r = room::register_room(txn, guild_id, None, Some(60_004), None, None).await?;
            rental::create_active_session(txn, guild_id, r.id, HOST).await?;

            // flow が申請前に行う「本人の既存セッション確認」が hit する。
            let existing = rental::find_active_session_for_user(txn, guild_id, HOST).await?;
            assert!(
                existing.is_some(),
                "既にレンタル中のユーザーは二重申請できないと判定される"
            );
            Ok(())
        })
    })
    .await;
}

/// 空き部屋が無いケース。
#[tokio::test]
#[ignore]
async fn no_available_rooms_when_all_occupied() {
    let guild_id: u64 = 999_500_000_000_000_005;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;
            let r = room::register_room(txn, guild_id, None, Some(60_005), None, None).await?;
            rental::create_active_session(txn, guild_id, r.id, HOST).await?;

            assert!(
                room::find_available_room(txn, guild_id).await?.is_none(),
                "唯一の部屋が使用中なら空きなし"
            );
            Ok(())
        })
    })
    .await;
}
