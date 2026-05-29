use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::BotResult;
use crate::facade::guild_config;
use crate::i18n::MessageKey;
use std::sync::Arc;
use twilight_model::http::attachment::Attachment;
use twilight_model::id::{Id, marker::GuildMarker};

pub async fn handle(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    lang: &str,
) -> BotResult<(String, Option<Attachment>)> {
    let yaml = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move { guild_config::dump(txn, guild_id.get()).await })
    })
    .await?;

    if yaml.trim().is_empty() {
        return Ok((
            state.i18n.get(lang, &MessageKey::BotConfigDownloadEmpty),
            None,
        ));
    }

    let attachment = Attachment::from_bytes("guild_config.yml".to_string(), yaml.into_bytes(), 0);
    Ok((String::new(), Some(attachment)))
}
