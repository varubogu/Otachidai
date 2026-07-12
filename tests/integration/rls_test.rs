//! RLS（Row Level Security）によるギルド分離の結合テスト。
//!
//! `guild` ロールは `app.current_guild_id` に一致する行だけを読み書きできる。
//! ここでは1トランザクション内でギルドコンテキストを切り替え、
//! あるギルドのデータが別ギルドからは **見えない/取れない** ことを検証する。
//!
//! `cargo test -- --ignored` で実行（実 PostgreSQL が必要。`guild` ロールは BYPASSRLS でないこと）。

use crate::integration::support::{set_guild, with_rollback_txn};
use otachidai::facade::{guild_settings, room};

const GUILD_A: u64 = 999_600_000_000_000_001;
const GUILD_B: u64 = 999_600_000_000_000_002;

/// ギルドAで作った部屋は、ギルドBのコンテキストからは一覧にも単体取得にも現れない。
#[tokio::test]
#[ignore]
async fn room_created_in_guild_a_is_invisible_to_guild_b() {
    with_rollback_txn(|txn| {
        Box::pin(async move {
            // --- ギルドA でデータ作成 ---
            set_guild(txn, GUILD_A).await;
            guild_settings::ensure_guild(txn, GUILD_A).await?;
            let room_a = room::register_room(txn, GUILD_A, None, Some(70_001), None, None).await?;

            // ギルドA からは見える。
            assert_eq!(room::list_rooms(txn, GUILD_A).await?.len(), 1);
            assert!(room::find_room_by_id(txn, room_a.id).await?.is_some());

            // --- ギルドB へ切り替え ---
            set_guild(txn, GUILD_B).await;

            // 一覧では見えない。
            assert!(
                room::list_rooms(txn, GUILD_B).await?.is_empty(),
                "RLS によりギルドB からはギルドA の部屋が一覧に出ない"
            );
            // RLS 頼みの単体取得（クエリに guild_id フィルタが無い find_room_by_id）でも取れない。
            assert!(
                room::find_room_by_id(txn, room_a.id).await?.is_none(),
                "RLS によりギルドB からはギルドA の部屋を id 指定でも取得できない"
            );
            // VC 指定でも取れない。
            assert!(
                room::find_room_by_voice_channel(txn, GUILD_B, 70_001)
                    .await?
                    .is_none()
            );

            // --- ギルドA に戻せば再び見える ---
            set_guild(txn, GUILD_A).await;
            assert!(room::find_room_by_id(txn, room_a.id).await?.is_some());
            Ok(())
        })
    })
    .await;
}

/// 各ギルドは自分のデータだけを数える（混ざらない）。
#[tokio::test]
#[ignore]
async fn list_rooms_is_scoped_per_guild() {
    with_rollback_txn(|txn| {
        Box::pin(async move {
            set_guild(txn, GUILD_A).await;
            guild_settings::ensure_guild(txn, GUILD_A).await?;
            room::register_room(txn, GUILD_A, None, Some(70_010), None, None).await?;
            room::register_room(txn, GUILD_A, None, Some(70_011), None, None).await?;

            set_guild(txn, GUILD_B).await;
            guild_settings::ensure_guild(txn, GUILD_B).await?;
            room::register_room(txn, GUILD_B, None, Some(70_020), None, None).await?;

            assert_eq!(room::list_rooms(txn, GUILD_B).await?.len(), 1);

            set_guild(txn, GUILD_A).await;
            assert_eq!(room::list_rooms(txn, GUILD_A).await?.len(), 2);
            Ok(())
        })
    })
    .await;
}
