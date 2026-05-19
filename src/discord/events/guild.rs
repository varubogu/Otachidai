use crate::app_state::AppState;
use crate::discord::commands::register::register_global_commands;
use crate::error::BotResult;
use std::sync::Arc;
use twilight_model::gateway::payload::incoming::{GuildCreate, Ready};

pub async fn handle_ready(state: Arc<AppState>, ready: Ready) -> BotResult<()> {
    tracing::info!(
        user = %ready.user.name,
        guilds = ready.guilds.len(),
        "Bot is ready"
    );
    register_global_commands(&state.http, state.application_id).await?;
    Ok(())
}

pub async fn handle_guild_create(state: Arc<AppState>, guild: Box<GuildCreate>) -> BotResult<()> {
    let (guild_id, available) = match guild.as_ref() {
        GuildCreate::Available(g) => {
            state.voice_occupancy.clear_guild(g.id.get());
            let mut tracked = 0usize;
            for voice_state in &g.voice_states {
                if let Some(channel_id) = voice_state.channel_id {
                    state.voice_occupancy.add_user(
                        g.id.get(),
                        voice_state.user_id.get(),
                        channel_id.get(),
                    );
                    tracked += 1;
                }
            }
            tracing::info!(
                guild_id = %g.id,
                voice_members = tracked,
                "GuildCreate received; voice occupancy populated"
            );
            (g.id, true)
        }
        GuildCreate::Unavailable(g) => (g.id, false),
    };

    // Voice occupancy is now populated; reconcile persisted rentals against it
    // so rentals orphaned by a restart get released or handed off correctly.
    if available && let Err(e) = crate::rental::reconcile::reconcile_guild(&state, guild_id).await {
        tracing::error!(%guild_id, "Rental reconciliation failed: {e}");
    }
    Ok(())
}
