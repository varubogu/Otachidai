//! YAML 一括設定の DB ラウンドトリップを検証する統合テスト。
//!
//! `cargo test -- --ignored` で実行（実 PostgreSQL が必要）。
//! 各テストは異なる `guild_id` を使うことで並列実行時の競合を避ける。

use otachidai::entities::{
    guild_channels, rental_question_presets, rental_routing_rules, room_groups, rooms,
};
use otachidai::facade::guild_config;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

fn db_url() -> String {
    let host = std::env::var("DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let user = std::env::var("GUILD_DB_USER").unwrap_or_else(|_| "otachidai_guild".to_string());
    let pass = std::env::var("GUILD_DB_PASSWORD").unwrap_or_default();
    let name = std::env::var("DB_NAME").unwrap_or_else(|_| "otachidai_db".to_string());
    format!("postgres://{user}:{pass}@{host}:{port}/{name}")
}

fn full_yaml() -> &'static str {
    r#"
version: 1
guild:
  language: ja
channels:
  report: "11111111111111111"
  rental_button: "22222222222222222"
  room_list: "33333333333333333"
  rental_post_fallback:
    channel: "44444444444444444"
    template: "{{user}} - {{answers}}"
question_presets:
  - name: 通常
    questions:
      - text: 目的
        answers: ["雑談", "作業"]
        routing_key: true
      - text: 備考
room_groups:
  - name: メイン
    channel_id: "55555555555555555"
rooms:
  - voice_channel_id: "66666666666666666"
    text_channel_id: "77777777777777777"
    group: メイン
    question_preset: 通常
routing_rules:
  - preset: 通常
    rules:
      - when: 雑談
        channel: "88888888888888888"
        template: "{{user}} が雑談を始めました（{{answer:備考}}）"
      - when: 作業
        channel: "99999999999999999"
"#
}

#[tokio::test]
#[ignore]
async fn apply_writes_full_config_to_db() {
    use sea_orm::Database;
    dotenvy::dotenv().ok();
    let db = Database::connect(db_url()).await.unwrap();

    let guild_id: u64 = 999_100_000_000_000_001;
    let validated = guild_config::parse(full_yaml()).expect("yaml should validate");

    otachidai::db::rls::with_guild_context(&db, guild_id, |txn| {
        Box::pin(async move {
            guild_config::apply(txn, guild_id, &validated).await?;

            let channels = guild_channels::Entity::find()
                .filter(guild_channels::Column::GuildId.eq(guild_id as i64))
                .all(txn)
                .await?;
            assert_eq!(channels.len(), 4, "all 4 channel rows should be inserted");

            let fallback = channels
                .iter()
                .find(|c| c.channel_type == guild_channels::CHANNEL_TYPE_RENTAL_POST_FALLBACK)
                .expect("fallback channel row");
            assert_eq!(fallback.channel_id, 44444444444444444);
            assert_eq!(
                fallback.template.as_deref(),
                Some("{{user}} - {{answers}}"),
                "fallback template should be persisted verbatim",
            );

            let presets = rental_question_presets::Entity::find()
                .filter(rental_question_presets::Column::GuildId.eq(guild_id as i64))
                .all(txn)
                .await?;
            assert_eq!(presets.len(), 1);
            let p = &presets[0];
            assert_eq!(p.name, "通常");
            assert_eq!(p.question_1.as_deref(), Some("目的"));
            assert_eq!(p.question_2.as_deref(), Some("備考"));
            assert_eq!(p.routing_key_index, Some(0));

            let groups = room_groups::Entity::find()
                .filter(room_groups::Column::GuildId.eq(guild_id as i64))
                .all(txn)
                .await?;
            assert_eq!(groups.len(), 1);
            assert_eq!(groups[0].channel_id, 55555555555555555);

            let room_rows = rooms::Entity::find()
                .filter(rooms::Column::GuildId.eq(guild_id as i64))
                .all(txn)
                .await?;
            assert_eq!(room_rows.len(), 1);
            assert_eq!(room_rows[0].voice_channel_id, Some(66666666666666666));
            assert_eq!(room_rows[0].text_channel_id, Some(77777777777777777));
            assert_eq!(room_rows[0].question_preset_id, Some(p.id));
            assert_eq!(room_rows[0].group_id, Some(groups[0].id));

            let rules = rental_routing_rules::Entity::find()
                .filter(rental_routing_rules::Column::GuildId.eq(guild_id as i64))
                .all(txn)
                .await?;
            assert_eq!(rules.len(), 2);
            for r in &rules {
                assert_eq!(r.preset_id, p.id);
            }
            let zatsudan = rules.iter().find(|r| r.match_value == "雑談").unwrap();
            assert_eq!(zatsudan.channel_id, 88888888888888888);
            assert!(zatsudan.template.is_some());
            let sagyou = rules.iter().find(|r| r.match_value == "作業").unwrap();
            assert_eq!(sagyou.channel_id, 99999999999999999);
            assert!(sagyou.template.is_none());

            Ok(())
        })
    })
    .await
    .unwrap();
}

#[tokio::test]
#[ignore]
async fn dump_then_parse_round_trips_through_db() {
    use sea_orm::Database;
    dotenvy::dotenv().ok();
    let db = Database::connect(db_url()).await.unwrap();

    let guild_id: u64 = 999_100_000_000_000_002;
    let validated = guild_config::parse(full_yaml()).unwrap();

    otachidai::db::rls::with_guild_context(&db, guild_id, |txn| {
        Box::pin(async move {
            guild_config::apply(txn, guild_id, &validated).await?;
            let dumped = guild_config::dump(txn, guild_id).await?;
            // The dumped YAML must always be re-parseable as a valid config.
            let reparsed = guild_config::parse(&dumped).expect("dump output should re-parse");
            assert_eq!(reparsed.presets.len(), 1);
            assert_eq!(reparsed.presets[0].routing_key_index, Some(0));
            assert_eq!(reparsed.rooms.len(), 1);
            assert_eq!(reparsed.groups.len(), 1);
            assert_eq!(reparsed.routing.len(), 2);
            assert!(reparsed.channels.rental_post_fallback.is_some());
            Ok(())
        })
    })
    .await
    .unwrap();
}

#[tokio::test]
#[ignore]
async fn apply_replaces_previous_state_fully() {
    use sea_orm::Database;
    dotenvy::dotenv().ok();
    let db = Database::connect(db_url()).await.unwrap();

    let guild_id: u64 = 999_100_000_000_000_003;
    let first = guild_config::parse(full_yaml()).unwrap();

    // Second config keeps only the report channel; everything else should be wiped.
    let second_yaml = r#"
version: 1
guild:
  language: en
channels:
  report: "10101010101010101"
"#;
    let second = guild_config::parse(second_yaml).unwrap();

    otachidai::db::rls::with_guild_context(&db, guild_id, |txn| {
        Box::pin(async move {
            guild_config::apply(txn, guild_id, &first).await?;
            guild_config::apply(txn, guild_id, &second).await?;

            let channels = guild_channels::Entity::find()
                .filter(guild_channels::Column::GuildId.eq(guild_id as i64))
                .all(txn)
                .await?;
            assert_eq!(channels.len(), 1, "only the report row should remain");
            assert_eq!(
                channels[0].channel_type,
                guild_channels::CHANNEL_TYPE_REPORT
            );
            assert_eq!(channels[0].channel_id, 10101010101010101);

            let presets = rental_question_presets::Entity::find()
                .filter(rental_question_presets::Column::GuildId.eq(guild_id as i64))
                .all(txn)
                .await?;
            assert!(presets.is_empty(), "presets should have been deleted");

            let groups = room_groups::Entity::find()
                .filter(room_groups::Column::GuildId.eq(guild_id as i64))
                .all(txn)
                .await?;
            assert!(groups.is_empty(), "groups should have been deleted");

            let room_rows = rooms::Entity::find()
                .filter(rooms::Column::GuildId.eq(guild_id as i64))
                .all(txn)
                .await?;
            assert!(room_rows.is_empty(), "rooms should have been deleted");

            let rules = rental_routing_rules::Entity::find()
                .filter(rental_routing_rules::Column::GuildId.eq(guild_id as i64))
                .all(txn)
                .await?;
            assert!(rules.is_empty(), "routing rules should have been deleted");

            let lang = otachidai::facade::guild_settings::get_language(txn, guild_id).await?;
            assert_eq!(lang, "en", "language change should be applied");

            Ok(())
        })
    })
    .await
    .unwrap();
}

#[tokio::test]
#[ignore]
async fn find_rooms_to_delete_returns_dropped_rooms() {
    use sea_orm::Database;
    dotenvy::dotenv().ok();
    let db = Database::connect(db_url()).await.unwrap();

    let guild_id: u64 = 999_100_000_000_000_004;
    let first = guild_config::parse(full_yaml()).unwrap();

    // Second config has a different VC id, so the existing room should be reported.
    let second_yaml = r#"
version: 1
rooms:
  - voice_channel_id: "12121212121212121"
"#;
    let second = guild_config::parse(second_yaml).unwrap();

    otachidai::db::rls::with_guild_context(&db, guild_id, |txn| {
        Box::pin(async move {
            guild_config::apply(txn, guild_id, &first).await?;
            let affected = guild_config::find_rooms_to_delete(txn, guild_id, &second).await?;
            assert_eq!(affected.len(), 1, "the previous room should be flagged");
            assert_eq!(affected[0].voice_channel_id, Some(66666666666666666));
            Ok(())
        })
    })
    .await
    .unwrap();
}
