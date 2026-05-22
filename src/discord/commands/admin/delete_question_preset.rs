use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::BotResult;
use crate::facade::question_preset;
use crate::i18n::MessageKey;
use std::sync::Arc;
use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};
use twilight_model::id::{Id, marker::GuildMarker};

pub async fn handle(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    data: &CommandData,
    lang: &str,
) -> BotResult<String> {
    let Some(name) = extract_optional_string(data, "name")
        .and_then(|name| question_preset::normalize_optional_text(&name))
    else {
        return Ok(state
            .i18n
            .get(lang, &MessageKey::AdminQuestionPresetNameRequired));
    };

    let deleted = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        let name = name.clone();
        Box::pin(async move { question_preset::delete_by_ref(txn, guild_id.get(), &name).await })
    })
    .await?;

    if deleted.is_some() {
        Ok(state
            .i18n
            .get(lang, &MessageKey::AdminQuestionPresetDeleted))
    } else {
        Ok(state
            .i18n
            .get(lang, &MessageKey::AdminQuestionPresetNotFound))
    }
}

fn extract_optional_string(data: &CommandData, name: &str) -> Option<String> {
    data.options.iter().find(|o| o.name == name).and_then(|o| {
        if let CommandOptionValue::String(value) = &o.value {
            Some(value.clone())
        } else {
            None
        }
    })
}
