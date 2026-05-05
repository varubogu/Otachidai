use crate::app_state::AppState;
use crate::error::BotResult;
use crate::i18n::MessageKey;
use std::sync::Arc;

pub async fn handle(state: Arc<AppState>, lang: &str) -> BotResult<String> {
    let title = state.i18n.get(lang, &MessageKey::HelpTitle);
    let user_section = state.i18n.get(lang, &MessageKey::HelpUser);
    let admin_section = state.i18n.get(lang, &MessageKey::HelpAdmin);

    Ok(format!("**{title}**\n\n{user_section}\n\n{admin_section}"))
}
