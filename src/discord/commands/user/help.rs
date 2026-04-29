use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::BotResult;
use crate::facade::guild_settings;
use crate::i18n::MessageKey;
use std::sync::Arc;
use twilight_model::id::{Id, marker::GuildMarker};

pub async fn handle(state: Arc<AppState>, guild_id: Id<GuildMarker>) -> BotResult<String> {
    let lang = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move { guild_settings::get_language(txn, guild_id.get()).await })
    })
    .await
    .unwrap_or_else(|_| "en".to_string());

    let title = state.i18n.get(&lang, &MessageKey::HelpTitle);
    let user_section = state.i18n.get(&lang, &MessageKey::HelpUser);
    let admin_section = state.i18n.get(&lang, &MessageKey::HelpAdmin);

    Ok(format!("**{title}**\n\n{user_section}\n\n{admin_section}"))
}
