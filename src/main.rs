use otachidai::{
    app_state::AppState,
    config::AppConfig,
    db::DbPools,
    i18n::I18n,
    rental::{state_machine::new_state_map, timeout::restore_pending_timeouts},
};
use std::sync::Arc;
use twilight_gateway::{
    EventTypeFlags, Intents, Shard, ShardId, StreamExt as _, error::ReceiveMessageErrorType,
};
use twilight_http::Client as HttpClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::from_filename(".env.app").ok();

    let config = AppConfig::from_env()?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.rust_log.parse().unwrap()),
        )
        .init();

    tracing::info!("Starting otachidai bot");

    let db = Arc::new(DbPools::new(&config).await?);

    let http = Arc::new(HttpClient::new(config.discord_token.clone()));

    let application_id = http.current_user_application().await?.model().await?.id;

    let i18n = Arc::new(I18n::load("locales")?);

    let rental_states = new_state_map();

    let state = Arc::new(AppState {
        db,
        http,
        application_id,
        i18n,
        rental_states,
    });

    // Restore any pending timeout tasks from DB
    restore_pending_timeouts(state.clone()).await;

    let intents = Intents::GUILD_VOICE_STATES | Intents::GUILDS;
    let mut shard = Shard::new(ShardId::ONE, config.discord_token, intents);

    tracing::info!("Connecting to Discord gateway");

    while let Some(event) = shard.next_event(EventTypeFlags::all()).await {
        match event {
            Ok(event) => {
                let state = state.clone();
                tokio::spawn(async move {
                    otachidai::discord::events::dispatcher::dispatch(state, event).await;
                });
            }
            Err(e) => {
                tracing::warn!("Gateway error: {e}");
                if matches!(e.kind(), ReceiveMessageErrorType::Reconnect) {
                    break;
                }
            }
        }
    }

    Ok(())
}
