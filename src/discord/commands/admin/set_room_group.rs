use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::BotResult;
use crate::facade::{group as group_facade, room as room_facade};
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
    let group_name = extract_string(data, "group")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if text_channel_id.is_none() && voice_channel_id.is_none() {
        return Ok(state.i18n.get(lang, &MessageKey::AdminRoomAtLeastOne));
    }

    let group_id = if let Some(name) = &group_name {
        let group = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
            let name = name.clone();
            Box::pin(
                async move { group_facade::find_group_by_name(txn, guild_id.get(), &name).await },
            )
        })
        .await?;
        match group {
            Some(group) => Some(group.id),
            None => {
                let mut args = FluentArgs::new();
                args.set("name", name.clone());
                return Ok(state.i18n.get_with_args(
                    lang,
                    &MessageKey::AdminGroupNotFound,
                    Some(&args),
                ));
            }
        }
    } else {
        None
    };

    let updated = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            room_facade::set_room_group(
                txn,
                guild_id.get(),
                text_channel_id,
                voice_channel_id,
                group_id,
            )
            .await
        })
    })
    .await?;

    if !updated {
        return Ok(state.i18n.get(lang, &MessageKey::AdminRoomNotFound));
    }

    crate::rental::status::trigger(&state, guild_id.get());

    match group_name {
        Some(name) => {
            let mut args = FluentArgs::new();
            args.set("name", name);
            Ok(state
                .i18n
                .get_with_args(lang, &MessageKey::AdminRoomGroupUpdated, Some(&args)))
        }
        None => Ok(state.i18n.get(lang, &MessageKey::AdminRoomGroupCleared)),
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
