use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::BotResult;
use crate::facade::question_preset::{self, QuestionInput};
use crate::i18n::MessageKey;
use std::sync::Arc;
use twilight_model::id::{Id, marker::GuildMarker};

/// Discord rejects messages longer than 2000 characters.
const MAX_MESSAGE_LEN: usize = 2000;

pub async fn handle(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    lang: &str,
) -> BotResult<String> {
    let presets = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move { question_preset::list_by_guild(txn, guild_id.get()).await })
    })
    .await?;

    if presets.is_empty() {
        return Ok(state
            .i18n
            .get(lang, &MessageKey::AdminQuestionPresetListEmpty));
    }

    let mut out = state
        .i18n
        .get(lang, &MessageKey::AdminQuestionPresetListHeader);

    for preset in &presets {
        out.push_str(&format!("\n\n[{}] {}", preset.id, preset.name));
        for (display_idx, q) in question_preset::model_questions_with_inputs(preset)
            .into_iter()
            .enumerate()
        {
            match q.input {
                QuestionInput::Dropdown(opts) => {
                    out.push_str(&format!(
                        "\n  {}. {} ({})",
                        display_idx + 1,
                        q.text,
                        opts.join(", ")
                    ));
                }
                QuestionInput::Text => {
                    out.push_str(&format!("\n  {}. {}", display_idx + 1, q.text));
                }
            }
        }
    }

    Ok(truncate(out))
}

fn truncate(mut s: String) -> String {
    if s.chars().count() <= MAX_MESSAGE_LEN {
        return s;
    }
    let cut: String = s.chars().take(MAX_MESSAGE_LEN - 1).collect();
    s = cut;
    s.push('…');
    s
}
