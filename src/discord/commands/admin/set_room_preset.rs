use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::BotResult;
use crate::facade::{question_preset, room as room_facade};
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
    let preset_ref = extract_string(data, "question_preset")
        .and_then(|name| question_preset::normalize_optional_text(&name));

    if text_channel_id.is_none() && voice_channel_id.is_none() {
        return Ok(state.i18n.get(lang, &MessageKey::AdminRoomAtLeastOne));
    }

    // Resolve the preset reference up front so we can report it before touching the room.
    let preset = if let Some(reference) = &preset_ref {
        let resolved = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
            let reference = reference.clone();
            Box::pin(
                async move { question_preset::find_by_ref(txn, guild_id.get(), &reference).await },
            )
        })
        .await?;
        match resolved {
            Some(preset) => Some(preset),
            None => {
                return Ok(state
                    .i18n
                    .get(lang, &MessageKey::AdminQuestionPresetNotFound));
            }
        }
    } else {
        None
    };

    let preset_id = preset.as_ref().map(|p| p.id);

    let updated = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            room_facade::set_room_preset(
                txn,
                guild_id.get(),
                text_channel_id,
                voice_channel_id,
                preset_id,
            )
            .await
        })
    })
    .await?;

    if !updated {
        return Ok(state.i18n.get(lang, &MessageKey::AdminRoomNotFound));
    }

    match preset {
        Some(preset) => {
            let mut args = FluentArgs::new();
            args.set("name", preset.name);
            Ok(state
                .i18n
                .get_with_args(lang, &MessageKey::AdminRoomPresetUpdated, Some(&args)))
        }
        None => Ok(state.i18n.get(lang, &MessageKey::AdminRoomPresetCleared)),
    }
}

fn extract_string(data: &CommandData, name: &str) -> Option<String> {
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
