use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::BotResult;
use crate::facade::room as room_facade;
use crate::language::resolve_language;
use crate::rental::{
    flow as rental_flow, handoff,
    state_machine::{RentalPromptMessage, RentalState, state_key},
};
use std::sync::Arc;
use twilight_model::channel::message::AllowedMentions;
use twilight_model::gateway::payload::incoming::VoiceStateUpdate;
use twilight_model::id::Id;

enum LeaveAction {
    CancelPending,
    ReleaseActive,
    StartHandoff { session_id: i32, room_id: i32 },
    Ignore,
}

pub async fn handle(state: Arc<AppState>, event: Box<VoiceStateUpdate>) -> BotResult<()> {
    let Some(guild_id) = event.guild_id else {
        return Ok(());
    };
    let user_id = event.user_id;
    let user_locale = event
        .member
        .as_ref()
        .and_then(|member| member.user.locale.as_deref())
        .map(ToOwned::to_owned);

    let previous_vc = state
        .voice_occupancy
        .channel_for_user(guild_id.get(), user_id.get());

    tracing::info!(
        %guild_id,
        %user_id,
        channel_id = ?event.channel_id.map(|c| c.get()),
        previous_vc = ?previous_vc,
        "VoiceStateUpdate received"
    );

    match event.channel_id {
        Some(channel_id) => {
            if let Some(vc_id) = previous_vc
                && vc_id != channel_id.get()
            {
                state
                    .voice_occupancy
                    .remove_user(guild_id.get(), user_id.get(), vc_id);
                let has_remaining_participants =
                    state.voice_occupancy.has_users(guild_id.get(), vc_id);
                handle_leave(
                    state.clone(),
                    guild_id,
                    user_id,
                    Id::new(vc_id),
                    has_remaining_participants,
                    user_locale.as_deref(),
                )
                .await?;
            }

            if previous_vc == Some(channel_id.get()) {
                state
                    .voice_occupancy
                    .add_user(guild_id.get(), user_id.get(), channel_id.get());
                return Ok(());
            }

            let was_empty = !state
                .voice_occupancy
                .has_users(guild_id.get(), channel_id.get());
            state
                .voice_occupancy
                .add_user(guild_id.get(), user_id.get(), channel_id.get());

            if was_empty {
                tracing::info!(%guild_id, %user_id, %channel_id, "User joined an empty voice channel; checking for rental room");
                handle_join(state, guild_id, user_id, channel_id, user_locale.as_deref()).await
            } else {
                tracing::info!(%guild_id, %user_id, %channel_id, "User joined a non-empty voice channel; not a new rental trigger");
                Ok(())
            }
        }
        None => {
            if let Some(vc_id) = previous_vc {
                state
                    .voice_occupancy
                    .remove_user(guild_id.get(), user_id.get(), vc_id);
                let has_remaining_participants =
                    state.voice_occupancy.has_users(guild_id.get(), vc_id);
                handle_leave(
                    state,
                    guild_id,
                    user_id,
                    Id::new(vc_id),
                    has_remaining_participants,
                    user_locale.as_deref(),
                )
                .await
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
    discord_locale: Option<&str>,
) -> BotResult<()> {
    let room = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            room_facade::find_room_by_voice_channel(txn, guild_id.get(), channel_id.get()).await
        })
    })
    .await?;

    let Some(room) = room else {
        tracing::info!(%guild_id, %channel_id, "Voice channel is not a registered rental room; ignoring join");
        return Ok(());
    };

    let key = state_key(guild_id, channel_id);
    if state.rental_states.contains_key(&key) {
        tracing::info!(%guild_id, %channel_id, "Rental already tracked for this VC; ignoring join");
        return Ok(());
    }

    let lang = resolve_language(&state, guild_id, discord_locale).await;
    tracing::info!(%guild_id, %user_id, %channel_id, %lang, discord_locale = ?discord_locale, "User joined empty rental VC");

    let result =
        rental_flow::start_rental(state.clone(), guild_id, user_id, Some(channel_id), &lang)
            .await?;

    if let rental_flow::StartRentalResult::AwaitingQuestions {
        session_id,
        room_id,
        ..
    } = result
    {
        let (content, components) =
            rental_flow::build_join_answer_button(&state, &lang, user_id, session_id, room_id);
        let allowed_mentions = AllowedMentions {
            users: vec![user_id],
            ..Default::default()
        };

        let notification_channel_id = prompt_channel_id(room.text_channel_id, channel_id);

        match state
            .http
            .create_message(notification_channel_id)
            .content(&content)
            .allowed_mentions(Some(&allowed_mentions))
            .components(&components)
            .await
        {
            Ok(response) => match response.model().await {
                Ok(message) => rental_flow::attach_purpose_prompt_message(
                    &state,
                    guild_id,
                    channel_id,
                    session_id,
                    RentalPromptMessage {
                        channel_id: notification_channel_id.get(),
                        message_id: message.id.get(),
                    },
                ),
                Err(err) => {
                    tracing::warn!(%guild_id, %user_id, %channel_id, %notification_channel_id, error = %err, "Failed to read rental question prompt message");
                    rental_flow::release_rental(state, guild_id, user_id, channel_id.get()).await?;
                }
            },
            Err(err) => {
                tracing::warn!(%guild_id, %user_id, %channel_id, %notification_channel_id, error = %err, "Failed to post rental question prompt");
                rental_flow::release_rental(state, guild_id, user_id, channel_id.get()).await?;
            }
        }
    }

    Ok(())
}

async fn handle_leave(
    state: Arc<AppState>,
    guild_id: twilight_model::id::Id<twilight_model::id::marker::GuildMarker>,
    user_id: twilight_model::id::Id<twilight_model::id::marker::UserMarker>,
    channel_id: twilight_model::id::Id<twilight_model::id::marker::ChannelMarker>,
    has_remaining_participants: bool,
    discord_locale: Option<&str>,
) -> BotResult<()> {
    let key = state_key(guild_id, channel_id);
    tracing::info!(%guild_id, %user_id, %channel_id, has_remaining_participants, "Handling voice channel leave");
    let action = {
        let entry = state.rental_states.get(&key);
        let Some(ref entry) = entry else {
            tracing::info!(%guild_id, %channel_id, "No rental state tracked for this VC; nothing to release");
            return Ok(());
        };
        match &entry.state {
            RentalState::AwaitingPurpose { host_user_id, .. } if *host_user_id == user_id.get() => {
                LeaveAction::CancelPending
            }
            RentalState::Active {
                session_id,
                host_user_id,
            } if *host_user_id == user_id.get() => {
                if has_remaining_participants {
                    LeaveAction::StartHandoff {
                        session_id: *session_id,
                        room_id: entry.room_id,
                    }
                } else {
                    LeaveAction::ReleaseActive
                }
            }
            _ => LeaveAction::Ignore,
        }
    }; // dashmap Ref dropped here

    match action {
        LeaveAction::CancelPending | LeaveAction::ReleaseActive => {
            tracing::info!(%guild_id, %user_id, %channel_id, "Rental host left; releasing rental");
            rental_flow::release_rental(state, guild_id, user_id, channel_id.get()).await?;
        }
        LeaveAction::StartHandoff {
            session_id,
            room_id,
        } => {
            tracing::info!(%guild_id, %user_id, %channel_id, session_id, "Rental host left with participants remaining; starting handoff");
            let lang = resolve_language(&state, guild_id, discord_locale).await;

            handoff::initiate_handoff(state, guild_id, channel_id, session_id, room_id, &lang)
                .await?;
        }
        LeaveAction::Ignore => {
            tracing::info!(%guild_id, %user_id, %channel_id, "Voice leave does not affect the tracked rental; ignoring");
        }
    }

    Ok(())
}
fn prompt_channel_id(
    text_channel_id: Option<i64>,
    voice_channel_id: twilight_model::id::Id<twilight_model::id::marker::ChannelMarker>,
) -> twilight_model::id::Id<twilight_model::id::marker::ChannelMarker> {
    text_channel_id
        .map(|id| Id::new(id as u64))
        .unwrap_or(voice_channel_id)
}

#[cfg(test)]
mod tests {
    use super::prompt_channel_id;
    use twilight_model::id::{Id, marker::ChannelMarker};

    #[test]
    fn prompt_channel_uses_paired_text_channel_when_registered() {
        let voice_channel_id = Id::<ChannelMarker>::new(100);

        assert_eq!(
            prompt_channel_id(Some(200), voice_channel_id),
            Id::<ChannelMarker>::new(200)
        );
    }

    #[test]
    fn prompt_channel_falls_back_to_voice_channel_chat() {
        let voice_channel_id = Id::<ChannelMarker>::new(100);

        assert_eq!(prompt_channel_id(None, voice_channel_id), voice_channel_id);
    }
}
