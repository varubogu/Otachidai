use crate::app_state::AppState;
use crate::error::BotResult;
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
    lang: &str,
) -> BotResult<InteractionResponse> {
    match start_rental(state.clone(), guild_id, user_id, None, lang).await? {
        Some((_session_id, _room_id, modal_response)) => Ok(modal_response),
        None => {
            let msg = state.i18n.get(lang, &MessageKey::BotRentalNoRooms);
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
