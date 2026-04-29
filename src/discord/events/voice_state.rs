use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::BotResult;
use crate::facade::{guild_settings, room as room_facade};
use crate::rental::{
    flow as rental_flow, handoff,
    state_machine::{RentalState, state_key},
};
use std::sync::Arc;
use twilight_model::gateway::payload::incoming::VoiceStateUpdate;
use twilight_model::id::Id;

pub async fn handle(state: Arc<AppState>, event: Box<VoiceStateUpdate>) -> BotResult<()> {
    let Some(guild_id) = event.guild_id else {
        return Ok(());
    };
    let user_id = event.user_id;

    match event.channel_id {
        Some(channel_id) => handle_join(state, guild_id, user_id, channel_id).await,
        None => {
            let left_vc = find_user_current_vc(&state, user_id.get());
            if let Some(vc_id) = left_vc {
                handle_leave(state, guild_id, user_id, Id::new(vc_id)).await
            } else {
                Ok(())
            }
        }
    }
}

async fn handle_join(
    state: Arc<AppState>,
    guild_id: twilight_model::id::Id<twilight_model::id::marker::GuildMarker>,
    user_id: twilight_model::id::Id<twilight_model::id::marker::UserMarker>,
    channel_id: twilight_model::id::Id<twilight_model::id::marker::ChannelMarker>,
) -> BotResult<()> {
    let room = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            room_facade::find_room_by_voice_channel(txn, guild_id.get(), channel_id.get()).await
        })
    })
    .await?;

    let Some(_room) = room else { return Ok(()) };

    let key = state_key(guild_id, channel_id);
    if state.rental_states.contains_key(&key) {
        return Ok(());
    }

    tracing::info!(%guild_id, %user_id, %channel_id, "User joined empty rental VC");

    let result =
        rental_flow::start_rental(state.clone(), guild_id, user_id, Some(channel_id)).await?;

    if let Some((_session_id, _room_id, _modal_response)) = result {
        let lang = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
            Box::pin(async move { guild_settings::get_language(txn, guild_id.get()).await })
        })
        .await
        .unwrap_or_else(|_| "en".to_string());

        let prompt = state
            .i18n
            .get(&lang, &crate::i18n::MessageKey::BotRentalRequestStart);

        if let Ok(dm) = state.http.create_private_channel(user_id).await
            && let Ok(ch) = dm.model().await
        {
            use crate::discord::components::rental_button::build_rental_button;
            let btn_label = state
                .i18n
                .get(&lang, &crate::i18n::MessageKey::RentButtonLabel);
            let _ = state
                .http
                .create_message(ch.id)
                .content(&prompt)
                .components(&build_rental_button(btn_label))
                .await;
        }
    }

    Ok(())
}

async fn handle_leave(
    state: Arc<AppState>,
    guild_id: twilight_model::id::Id<twilight_model::id::marker::GuildMarker>,
    user_id: twilight_model::id::Id<twilight_model::id::marker::UserMarker>,
    channel_id: twilight_model::id::Id<twilight_model::id::marker::ChannelMarker>,
) -> BotResult<()> {
    let key = state_key(guild_id, channel_id);
    let (session_id, room_id, is_host_leaving) = {
        let entry = state.rental_states.get(&key);
        let Some(ref entry) = entry else {
            return Ok(());
        };
        let is_host = matches!(&entry.state, RentalState::Active { host_user_id, .. } if *host_user_id == user_id.get());
        (entry.session_id(), entry.room_id, is_host)
    }; // dashmap Ref dropped here

    if is_host_leaving {
        let lang = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
            Box::pin(async move { guild_settings::get_language(txn, guild_id.get()).await })
        })
        .await
        .unwrap_or_else(|_| "en".to_string());

        handoff::initiate_handoff(state, guild_id, channel_id, session_id, room_id, &lang).await?;
    }

    Ok(())
}

fn find_user_current_vc(state: &AppState, user_id: u64) -> Option<u64> {
    for entry in state.rental_states.iter() {
        if let RentalState::Active { host_user_id, .. } = &entry.state
            && *host_user_id == user_id
        {
            return Some(entry.key().1);
        }
    }
    None
}
