use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::BotResult;
use crate::facade::{group as group_facade, guild_settings, question_preset, room as room_facade};
use crate::i18n::MessageKey;
use fluent_bundle::FluentArgs;
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
    let text_channel_id = extract_optional_channel(data, "text_channel");
    let voice_channel_id = extract_optional_channel(data, "voice_channel");
    let question_preset_name = extract_optional_string(data, "question_preset")
        .and_then(|name| question_preset::normalize_optional_text(&name));
    let group_name = extract_optional_string(data, "group")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if text_channel_id.is_none() && voice_channel_id.is_none() {
        return Ok(state.i18n.get(lang, &MessageKey::AdminRoomAtLeastOne));
    }

    let (question_preset_id, group_id) =
        with_guild_context(&state.db.guild, guild_id.get(), |txn| {
            let question_preset_name = question_preset_name.clone();
            let group_name = group_name.clone();
            Box::pin(async move {
                guild_settings::ensure_guild(txn, guild_id.get()).await?;
                let question_preset_id = if let Some(name) = question_preset_name {
                    question_preset::find_by_name(txn, guild_id.get(), &name)
                        .await?
                        .map(|preset| preset.id)
                } else {
                    None
                };
                let group_id = if let Some(name) = group_name {
                    group_facade::find_group_by_name(txn, guild_id.get(), &name)
                        .await?
                        .map(|group| group.id)
                } else {
                    None
                };
                Ok((question_preset_id, group_id))
            })
        })
        .await?;

    if question_preset_name.is_some() && question_preset_id.is_none() {
        return Ok(state
            .i18n
            .get(lang, &MessageKey::AdminQuestionPresetNotFound));
    }

    if group_name.is_some() && group_id.is_none() {
        let mut args = FluentArgs::new();
        args.set("name", group_name.unwrap_or_default());
        return Ok(state
            .i18n
            .get_with_args(lang, &MessageKey::AdminGroupNotFound, Some(&args)));
    }

    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            room_facade::register_room(
                txn,
                guild_id.get(),
                text_channel_id,
                voice_channel_id,
                question_preset_id,
                group_id,
            )
            .await
            .map(|_| ())
        })
    })
    .await?;

    crate::rental::status::trigger(&state, guild_id.get());

    Ok(state.i18n.get(lang, &MessageKey::AdminRoomRegistered))
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

fn extract_optional_channel(data: &CommandData, name: &str) -> Option<u64> {
    data.options.iter().find(|o| o.name == name).and_then(|o| {
        if let CommandOptionValue::Channel(id) = &o.value {
            Some(id.get())
        } else {
            None
        }
    })
}
