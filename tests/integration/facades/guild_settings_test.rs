use otachidai::facade::guild_settings;

fn db_url() -> String {
    let host = std::env::var("DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string());
    let user = std::env::var("GUILD_DB_USER").unwrap_or_else(|_| "otachidai_guild".to_string());
    let pass = std::env::var("GUILD_DB_PASSWORD").unwrap_or_default();
    let name = std::env::var("DB_NAME").unwrap_or_else(|_| "otachidai_db".to_string());
    format!("postgres://{user}:{pass}@{host}:{port}/{name}")
}

#[tokio::test]
#[ignore]
async fn test_ensure_guild_creates_entry() {
    use sea_orm::Database;
    dotenvy::dotenv().ok();
    let db = Database::connect(db_url()).await.unwrap();

    let guild_id: u64 = 999_000_000_000_000_001;

    otachidai::db::rls::with_guild_context(&db, guild_id, |txn| {
        Box::pin(async move {
            let guild = guild_settings::ensure_guild(txn, guild_id).await.unwrap();
            assert_eq!(guild.guild_id, guild_id as i64);
            assert_eq!(guild.language, "en");
            Ok(())
        })
    })
    .await
    .unwrap();
}

#[tokio::test]
#[ignore]
async fn test_set_and_get_report_channel() {
    use sea_orm::Database;
    dotenvy::dotenv().ok();
    let db = Database::connect(db_url()).await.unwrap();

    let guild_id: u64 = 999_000_000_000_000_002;
    let channel_id: u64 = 123_456_789;

    otachidai::db::rls::with_guild_context(&db, guild_id, |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id).await.unwrap();
            guild_settings::set_report_channel(txn, guild_id, channel_id)
                .await
                .unwrap();
            let retrieved = guild_settings::get_report_channel(txn, guild_id)
                .await
                .unwrap();
            assert_eq!(retrieved, Some(channel_id as i64));
            Ok(())
        })
    })
    .await
    .unwrap();
}
