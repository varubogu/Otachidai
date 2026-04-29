use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::BotResult;
use crate::facade::guild_settings;
use crate::i18n::MessageKey;
use crate::rental::flow::start_rental;
use std::sync::Arc;
use twilight_model::channel::message::MessageFlags;
use twilight_model::http::interaction::{
    InteractionResponse, InteractionResponseData, InteractionResponseType,
};
use twilight_model::id::{
    Id,
    marker::{GuildMarker, UserMarker},
};

pub async fn handle(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
) -> BotResult<InteractionResponse> {
    let lang = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move { guild_settings::get_language(txn, guild_id.get()).await })
    })
    .await
    .unwrap_or_else(|_| "en".to_string());

    match start_rental(state.clone(), guild_id, user_id, None).await? {
        Some((_session_id, _room_id, modal_response)) => Ok(modal_response),
        None => {
            let msg = state.i18n.get(&lang, &MessageKey::BotRentalNoRooms);
            Ok(InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(InteractionResponseData {
                    content: Some(msg),
                    flags: Some(MessageFlags::EPHEMERAL),
                    ..Default::default()
                }),
            })
        }
    }
}
