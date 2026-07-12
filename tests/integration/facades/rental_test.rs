//! `src/facade/rental.rs` の結合テスト。
//!
//! `cargo test -- --ignored` で実行（実 PostgreSQL が必要）。
//! 各テストは [`crate::integration::support::with_test_guild`] のトランザクション内で走り、
//! 終了時に rollback されるため互いに干渉しない。

use crate::integration::support::{seed_room, with_test_guild};
use otachidai::entities::rental_sessions::{
    STATE_ACTIVE, STATE_AWAITING_PURPOSE, STATE_PENDING_HANDOFF, STATE_RELEASED,
};
use otachidai::entities::scheduled_tasks;
use otachidai::facade::rental;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

const HOST: u64 = 1001;

#[tokio::test]
#[ignore]
async fn create_session_sets_awaiting_purpose_and_schedules_timeout() {
    let guild_id: u64 = 999_200_000_000_000_001;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            let room_id = seed_room(txn, guild_id, 50_001).await?;
            let session = rental::create_session(txn, guild_id, room_id, HOST).await?;

            assert_eq!(session.state, STATE_AWAITING_PURPOSE);
            assert_eq!(session.room_id, room_id);
            assert_eq!(session.host_user_id, HOST as i64);
            assert!(session.purpose.is_none());
            assert!(
                session.purpose_deadline.is_some(),
                "awaiting セッションには目的入力の締切が設定される"
            );
            assert!(session.ended_at.is_none());

            // 10分後のタイムアウト通知タスクが未処理状態で作られる。
            let tasks = scheduled_tasks::Entity::find()
                .filter(scheduled_tasks::Column::RentalSessionId.eq(session.id))
                .all(txn)
                .await?;
            assert_eq!(tasks.len(), 1);
            assert_eq!(
                tasks[0].task_type,
                scheduled_tasks::TASK_TYPE_TIMEOUT_NOTIFICATION
            );
            assert!(!tasks[0].processed);
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn create_active_session_skips_purpose_phase() {
    let guild_id: u64 = 999_200_000_000_000_002;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            let room_id = seed_room(txn, guild_id, 50_002).await?;
            let session = rental::create_active_session(txn, guild_id, room_id, HOST).await?;

            assert_eq!(session.state, STATE_ACTIVE);
            assert!(
                session.purpose_deadline.is_none(),
                "目的不要なので締切は無い"
            );
            // 即アクティブのセッションにはタイムアウトタスクが作られない。
            let tasks = scheduled_tasks::Entity::find()
                .filter(scheduled_tasks::Column::RentalSessionId.eq(session.id))
                .all(txn)
                .await?;
            assert!(tasks.is_empty());
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn set_purpose_transitions_to_active_and_clears_deadline() {
    let guild_id: u64 = 999_200_000_000_000_003;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            let room_id = seed_room(txn, guild_id, 50_003).await?;
            let session = rental::create_session(txn, guild_id, room_id, HOST).await?;

            let updated = rental::set_purpose(txn, session.id, "雑談したい".to_string()).await?;
            assert_eq!(updated.state, STATE_ACTIVE);
            assert_eq!(updated.purpose.as_deref(), Some("雑談したい"));
            assert!(updated.purpose_deadline.is_none());
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn set_session_room_reassigns_room() {
    let guild_id: u64 = 999_200_000_000_000_004;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            let room_a = seed_room(txn, guild_id, 50_004).await?;
            let room_b = otachidai::facade::room::register_room(
                txn,
                guild_id,
                None,
                Some(50_005),
                None,
                None,
            )
            .await?
            .id;
            let session = rental::create_session(txn, guild_id, room_a, HOST).await?;

            rental::set_session_room(txn, session.id, room_b).await?;

            let reloaded = otachidai::entities::rental_sessions::Entity::find_by_id(session.id)
                .one(txn)
                .await?
                .expect("session exists");
            assert_eq!(reloaded.room_id, room_b);
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn release_session_marks_released_with_end_time() {
    let guild_id: u64 = 999_200_000_000_000_005;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            let room_id = seed_room(txn, guild_id, 50_006).await?;
            let session = rental::create_active_session(txn, guild_id, room_id, HOST).await?;

            rental::release_session(txn, session.id).await?;

            let reloaded = otachidai::entities::rental_sessions::Entity::find_by_id(session.id)
                .one(txn)
                .await?
                .expect("session exists");
            assert_eq!(reloaded.state, STATE_RELEASED);
            assert!(reloaded.ended_at.is_some());
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn transfer_host_changes_host_and_activates() {
    let guild_id: u64 = 999_200_000_000_000_006;
    let new_host: u64 = 2002;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            let room_id = seed_room(txn, guild_id, 50_007).await?;
            let session = rental::create_active_session(txn, guild_id, room_id, HOST).await?;
            rental::set_pending_handoff(txn, session.id).await?;

            let transferred = rental::transfer_host(txn, session.id, new_host).await?;
            assert_eq!(transferred.host_user_id, new_host as i64);
            assert_eq!(transferred.state, STATE_ACTIVE);
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn set_pending_handoff_transitions_state() {
    let guild_id: u64 = 999_200_000_000_000_007;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            let room_id = seed_room(txn, guild_id, 50_008).await?;
            let session = rental::create_active_session(txn, guild_id, room_id, HOST).await?;

            rental::set_pending_handoff(txn, session.id).await?;

            let reloaded = otachidai::entities::rental_sessions::Entity::find_by_id(session.id)
                .one(txn)
                .await?
                .expect("session exists");
            assert_eq!(reloaded.state, STATE_PENDING_HANDOFF);
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn find_active_session_for_room_matches_live_states_only() {
    let guild_id: u64 = 999_200_000_000_000_008;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            let room_id = seed_room(txn, guild_id, 50_009).await?;
            let session = rental::create_active_session(txn, guild_id, room_id, HOST).await?;

            // active なら見つかる
            let found = rental::find_active_session_for_room(txn, room_id).await?;
            assert_eq!(found.map(|s| s.id), Some(session.id));

            // release すると見つからない
            rental::release_session(txn, session.id).await?;
            let after_release = rental::find_active_session_for_room(txn, room_id).await?;
            assert!(after_release.is_none());
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn find_active_session_for_user_excludes_pending_handoff() {
    let guild_id: u64 = 999_200_000_000_000_009;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            let room_id = seed_room(txn, guild_id, 50_010).await?;
            let session = rental::create_active_session(txn, guild_id, room_id, HOST).await?;

            // active のうちは本人のセッションとして見つかる
            let found = rental::find_active_session_for_user(txn, guild_id, HOST).await?;
            assert_eq!(found.map(|s| s.id), Some(session.id));

            // pending_handoff は「本人がレンタル中」の対象外
            rental::set_pending_handoff(txn, session.id).await?;
            let after = rental::find_active_session_for_user(txn, guild_id, HOST).await?;
            assert!(
                after.is_none(),
                "pending_handoff は二重申請判定の対象外であること"
            );
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn find_active_sessions_by_guild_lists_all_live_states() {
    let guild_id: u64 = 999_200_000_000_000_010;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            let room_a = seed_room(txn, guild_id, 50_011).await?;
            let room_b = otachidai::facade::room::register_room(
                txn,
                guild_id,
                None,
                Some(50_012),
                None,
                None,
            )
            .await?
            .id;
            let room_c = otachidai::facade::room::register_room(
                txn,
                guild_id,
                None,
                Some(50_013),
                None,
                None,
            )
            .await?
            .id;

            let s_await = rental::create_session(txn, guild_id, room_a, HOST).await?;
            let s_active = rental::create_active_session(txn, guild_id, room_b, 3003).await?;
            let s_released = rental::create_active_session(txn, guild_id, room_c, 4004).await?;
            rental::release_session(txn, s_released.id).await?;

            let live = rental::find_active_sessions_by_guild(txn, guild_id).await?;
            let ids: Vec<i32> = live.iter().map(|s| s.id).collect();
            assert!(ids.contains(&s_await.id));
            assert!(ids.contains(&s_active.id));
            assert!(
                !ids.contains(&s_released.id),
                "released は live 一覧に含めない"
            );
            assert_eq!(live.len(), 2);
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn mark_session_tasks_processed_flags_all_pending_tasks() {
    let guild_id: u64 = 999_200_000_000_000_011;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            let room_id = seed_room(txn, guild_id, 50_014).await?;
            let session = rental::create_session(txn, guild_id, room_id, HOST).await?;

            rental::mark_session_tasks_processed(txn, session.id).await?;

            let tasks = scheduled_tasks::Entity::find()
                .filter(scheduled_tasks::Column::RentalSessionId.eq(session.id))
                .all(txn)
                .await?;
            assert!(!tasks.is_empty());
            assert!(
                tasks.iter().all(|t| t.processed),
                "セッションに紐づく全タスクが処理済みになる"
            );
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn mark_task_processed_flags_single_task() {
    let guild_id: u64 = 999_200_000_000_000_012;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            let room_id = seed_room(txn, guild_id, 50_015).await?;
            let session = rental::create_session(txn, guild_id, room_id, HOST).await?;
            let task = scheduled_tasks::Entity::find()
                .filter(scheduled_tasks::Column::RentalSessionId.eq(session.id))
                .one(txn)
                .await?
                .expect("timeout task exists");

            rental::mark_task_processed(txn, task.id).await?;

            let reloaded = scheduled_tasks::Entity::find_by_id(task.id)
                .one(txn)
                .await?
                .expect("task exists");
            assert!(reloaded.processed);
            Ok(())
        })
    })
    .await;
}
