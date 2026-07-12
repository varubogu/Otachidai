//! `src/facade/room.rs` の結合テスト。
//!
//! `cargo test -- --ignored` で実行（実 PostgreSQL が必要）。

use crate::integration::support::with_test_guild;
use otachidai::facade::guild_settings;
use otachidai::facade::rental;
use otachidai::facade::room;

const HOST: u64 = 7001;

#[tokio::test]
#[ignore]
async fn register_room_defaults_to_available() {
    let guild_id: u64 = 999_300_000_000_000_001;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;
            let r =
                room::register_room(txn, guild_id, Some(80_001), Some(80_002), None, None).await?;
            assert_eq!(r.guild_id, guild_id as i64);
            assert_eq!(r.voice_channel_id, Some(80_002));
            assert_eq!(r.text_channel_id, Some(80_001));
            assert!(r.is_available, "新規部屋は既定で利用可能");
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn find_room_by_voice_channel_matches_guild_and_channel() {
    let guild_id: u64 = 999_300_000_000_000_002;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;
            let r = room::register_room(txn, guild_id, None, Some(80_010), None, None).await?;

            let found = room::find_room_by_voice_channel(txn, guild_id, 80_010).await?;
            assert_eq!(found.map(|m| m.id), Some(r.id));

            let missing = room::find_room_by_voice_channel(txn, guild_id, 99_999).await?;
            assert!(missing.is_none());
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn find_available_room_skips_rooms_with_active_session() {
    let guild_id: u64 = 999_300_000_000_000_003;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;
            // id 昇順で room_a が先。room_a を埋めると room_b が返る。
            let room_a = room::register_room(txn, guild_id, None, Some(80_020), None, None).await?;
            let room_b = room::register_room(txn, guild_id, None, Some(80_021), None, None).await?;

            // 何も埋まっていなければ最初の部屋が返る。
            let first = room::find_available_room(txn, guild_id).await?;
            assert_eq!(first.map(|r| r.id), Some(room_a.id));

            // room_a にアクティブセッションを張ると room_b が返る。
            rental::create_active_session(txn, guild_id, room_a.id, HOST).await?;
            let next = room::find_available_room(txn, guild_id).await?;
            assert_eq!(next.map(|r| r.id), Some(room_b.id));

            // 両方埋めると空きなし。
            rental::create_active_session(txn, guild_id, room_b.id, HOST + 1).await?;
            let none = room::find_available_room(txn, guild_id).await?;
            assert!(none.is_none());
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn list_available_rooms_excludes_active_and_returns_rest() {
    let guild_id: u64 = 999_300_000_000_000_004;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;
            let room_a = room::register_room(txn, guild_id, None, Some(80_030), None, None).await?;
            let room_b = room::register_room(txn, guild_id, None, Some(80_031), None, None).await?;
            let room_c = room::register_room(txn, guild_id, None, Some(80_032), None, None).await?;

            rental::create_active_session(txn, guild_id, room_b.id, HOST).await?;

            let available = room::list_available_rooms(txn, guild_id).await?;
            let ids: Vec<i32> = available.iter().map(|r| r.id).collect();
            assert!(ids.contains(&room_a.id));
            assert!(ids.contains(&room_c.id));
            assert!(!ids.contains(&room_b.id));
            assert_eq!(available.len(), 2);
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn set_room_availability_toggles_flag() {
    let guild_id: u64 = 999_300_000_000_000_005;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;
            let r = room::register_room(txn, guild_id, None, Some(80_040), None, None).await?;

            room::set_room_availability(txn, r.id, false).await?;
            let after = room::find_room_by_id(txn, r.id)
                .await?
                .expect("room exists");
            assert!(!after.is_available);

            room::set_room_availability(txn, r.id, true).await?;
            let restored = room::find_room_by_id(txn, r.id)
                .await?
                .expect("room exists");
            assert!(restored.is_available);
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn set_room_group_returns_false_when_no_match() {
    let guild_id: u64 = 999_300_000_000_000_006;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;
            room::register_room(txn, guild_id, None, Some(80_050), None, None).await?;

            // 存在する部屋なら true、group_id がクリアされる（None 指定）。
            let matched = room::set_room_group(txn, guild_id, None, Some(80_050), None).await?;
            assert!(matched);

            // 一致する部屋が無ければ false。
            let unmatched = room::set_room_group(txn, guild_id, None, Some(99_999), None).await?;
            assert!(!unmatched);
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn set_room_preset_returns_false_when_no_match() {
    let guild_id: u64 = 999_300_000_000_000_007;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;
            room::register_room(txn, guild_id, None, Some(80_060), None, None).await?;

            let matched = room::set_room_preset(txn, guild_id, None, Some(80_060), None).await?;
            assert!(matched);

            let unmatched = room::set_room_preset(txn, guild_id, None, Some(99_999), None).await?;
            assert!(!unmatched);
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn delete_room_removes_matching_room() {
    let guild_id: u64 = 999_300_000_000_000_008;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;
            let r = room::register_room(txn, guild_id, None, Some(80_070), None, None).await?;

            let deleted = room::delete_room(txn, guild_id, None, Some(80_070)).await?;
            assert!(deleted);
            assert!(room::find_room_by_id(txn, r.id).await?.is_none());

            // もう一度消そうとしても一致なしで false。
            let again = room::delete_room(txn, guild_id, None, Some(80_070)).await?;
            assert!(!again);
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn list_rooms_and_ungrouped_rooms() {
    let guild_id: u64 = 999_300_000_000_000_009;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;
            room::register_room(txn, guild_id, None, Some(80_080), None, None).await?;
            room::register_room(txn, guild_id, None, Some(80_081), None, None).await?;

            let all = room::list_rooms(txn, guild_id).await?;
            assert_eq!(all.len(), 2);

            // どちらも group 未設定なので ungrouped に両方出る。
            let ungrouped = room::list_ungrouped_rooms(txn, guild_id).await?;
            assert_eq!(ungrouped.len(), 2);
            Ok(())
        })
    })
    .await;
}
