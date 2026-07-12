//! `src/facade/question_preset.rs` の DB 側関数の結合テスト。
//! CSV パース（`parse_answer_options`）は同ファイルのユニットテストで担保済み。
//!
//! `cargo test -- --ignored` で実行（実 PostgreSQL が必要）。

use crate::integration::support::with_test_guild;
use otachidai::facade::guild_settings;
use otachidai::facade::question_preset as qp;

fn q(items: &[&str]) -> Vec<Option<String>> {
    items.iter().map(|s| Some(s.to_string())).collect()
}

#[tokio::test]
#[ignore]
async fn upsert_preset_inserts_then_updates_in_place() {
    let guild_id: u64 = 999_400_000_000_000_001;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;

            // 新規作成
            let created = qp::upsert_preset(
                txn,
                guild_id,
                "通常".to_string(),
                q(&["目的", "備考"]),
                vec![Some("雑談,作業".to_string()), None],
            )
            .await?;
            assert_eq!(created.name, "通常");
            assert_eq!(created.question_1.as_deref(), Some("目的"));
            assert_eq!(created.question_2.as_deref(), Some("備考"));
            assert_eq!(created.answer_1.as_deref(), Some("雑談,作業"));

            // 同名で upsert すると同一行が更新される（行は増えない）。
            let updated = qp::upsert_preset(
                txn,
                guild_id,
                "通常".to_string(),
                q(&["新しい目的"]),
                vec![None],
            )
            .await?;
            assert_eq!(updated.id, created.id, "同名 upsert は同じ行を更新する");
            assert_eq!(updated.question_1.as_deref(), Some("新しい目的"));
            assert!(
                updated.question_2.is_none(),
                "更新時に余分な質問はクリアされる"
            );

            let all = qp::list_by_guild(txn, guild_id).await?;
            assert_eq!(all.len(), 1, "upsert なので行は増えない");
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn list_by_guild_orders_by_id() {
    let guild_id: u64 = 999_400_000_000_000_002;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;
            qp::upsert_preset(txn, guild_id, "A".to_string(), q(&["q"]), vec![None]).await?;
            qp::upsert_preset(txn, guild_id, "B".to_string(), q(&["q"]), vec![None]).await?;

            let list = qp::list_by_guild(txn, guild_id).await?;
            assert_eq!(list.len(), 2);
            assert!(list[0].id < list[1].id, "id 昇順で並ぶ");
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn find_by_id_name_and_ref_resolve_correctly() {
    let guild_id: u64 = 999_400_000_000_000_003;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;
            let preset =
                qp::upsert_preset(txn, guild_id, "通常".to_string(), q(&["q"]), vec![None]).await?;

            assert_eq!(
                qp::find_by_id(txn, preset.id).await?.map(|p| p.id),
                Some(preset.id)
            );
            assert_eq!(
                qp::find_by_name(txn, guild_id, "通常").await?.map(|p| p.id),
                Some(preset.id)
            );
            // "id:name" 形式
            let by_ref = qp::find_by_ref(txn, guild_id, &qp::format_ref_label(&preset)).await?;
            assert_eq!(by_ref.map(|p| p.id), Some(preset.id));
            // 名前だけでも解決
            let by_name_ref = qp::find_by_ref(txn, guild_id, "通常").await?;
            assert_eq!(by_name_ref.map(|p| p.id), Some(preset.id));
            // 該当なし
            assert!(
                qp::find_by_name(txn, guild_id, "存在しない")
                    .await?
                    .is_none()
            );
            Ok(())
        })
    })
    .await;
}

#[tokio::test]
#[ignore]
async fn delete_by_ref_removes_preset() {
    let guild_id: u64 = 999_400_000_000_000_004;
    with_test_guild(guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await?;
            let preset =
                qp::upsert_preset(txn, guild_id, "消す".to_string(), q(&["q"]), vec![None]).await?;

            let deleted = qp::delete_by_ref(txn, guild_id, "消す").await?;
            assert_eq!(deleted.map(|p| p.id), Some(preset.id));
            assert!(qp::find_by_id(txn, preset.id).await?.is_none());

            // 二度目は該当なし。
            let again = qp::delete_by_ref(txn, guild_id, "消す").await?;
            assert!(again.is_none());
            Ok(())
        })
    })
    .await;
}
