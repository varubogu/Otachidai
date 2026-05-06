use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::BotResult;
use crate::facade::{guild_settings, question_preset};
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

    let questions = (1..=10)
        .map(|index| extract_optional_string(data, &format!("question_{index}")))
        .collect::<Vec<_>>();

    if questions.iter().all(|question| {
        question
            .as_deref()
            .and_then(question_preset::normalize_optional_text)
            .is_none()
    }) {
        return Ok(state
            .i18n
            .get(lang, &MessageKey::AdminQuestionPresetAtLeastOne));
    }

    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        let name = name.clone();
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id.get()).await?;
            question_preset::upsert_preset(txn, guild_id.get(), name, questions)
                .await
                .map(|_| ())
        })
    })
    .await?;

    Ok(state.i18n.get(lang, &MessageKey::AdminQuestionPresetSaved))
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
