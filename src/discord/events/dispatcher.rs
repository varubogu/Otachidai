use crate::app_state::AppState;
use std::sync::Arc;
use twilight_gateway::Event;

pub async fn dispatch(state: Arc<AppState>, event: Event) {
    let result = match event {
        Event::Ready(ready) => super::guild::handle_ready(state, ready).await,
        Event::InteractionCreate(interaction) => {
            super::interaction::handle(state, *interaction).await
        }
        Event::VoiceStateUpdate(voice_state) => {
            super::voice_state::handle(state, voice_state).await
        }
        Event::GuildCreate(guild) => super::guild::handle_guild_create(state, guild).await,
        _ => return,
    };

    if let Err(e) = result {
        tracing::error!("Error handling event: {e}");
    }
}
