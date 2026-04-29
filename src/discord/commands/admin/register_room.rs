use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::BotResult;
use crate::facade::{guild_settings, room as room_facade};
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
) -> BotResult<String> {
    let text_channel_id = extract_optional_channel(data, "text_channel");
    let voice_channel_id = extract_optional_channel(data, "voice_channel");

    if text_channel_id.is_none() && voice_channel_id.is_none() {
        let lang = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
            Box::pin(async move { guild_settings::get_language(txn, guild_id.get()).await })
        })
        .await
        .unwrap_or_else(|_| "en".to_string());
        return Ok(state.i18n.get(&lang, &MessageKey::AdminRoomAtLeastOne));
    }

    let lang = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id.get()).await?;
            guild_settings::get_language(txn, guild_id.get()).await
        })
    })
    .await
    .unwrap_or_else(|_| "en".to_string());

    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            room_facade::register_room(txn, guild_id.get(), text_channel_id, voice_channel_id)
                .await
                .map(|_| ())
        })
    })
    .await?;

    Ok(state.i18n.get(&lang, &MessageKey::AdminRoomRegistered))
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
