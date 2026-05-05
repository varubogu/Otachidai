use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::entities::{rental_sessions, rooms};
use crate::error::BotResult;
use crate::facade::{rental as rental_facade, room as room_facade};
use crate::i18n::MessageKey;
use crate::language::resolve_language;
use crate::rental::state_machine::{RentalState, RentalStateEntry};
use crate::rental::timeout::spawn_purpose_timeout;
use fluent_bundle::FluentArgs;
use sea_orm::EntityTrait;
use std::sync::Arc;
use std::time::Duration;
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker, UserMarker},
};
use twilight_model::{
    channel::message::component::{ActionRow, Component, TextInput, TextInputStyle},
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
};

pub enum StartRentalResult {
    Started {
        session_id: i32,
        room_id: i32,
        response: InteractionResponse,
    },
    AlreadyRenting,
    NoAvailableRooms,
}

pub fn build_purpose_modal(
    state: &AppState,
    lang: &str,
    session_id: i32,
    room_id: i32,
) -> InteractionResponse {
    let title = state.i18n.get(lang, &MessageKey::BotRentalRequestStart);
    let label = state.i18n.get(lang, &MessageKey::BotRentalPurposeLabel);

    InteractionResponse {
        kind: InteractionResponseType::Modal,
        data: Some(InteractionResponseData {
            custom_id: Some(format!("purpose_modal:{session_id}:{room_id}")),
            title: Some(title),
            components: Some(vec![Component::ActionRow(ActionRow {
                id: None,
                components: vec![Component::TextInput(TextInput {
                    id: None,
                    custom_id: "purpose_text".to_string(),
                    #[allow(deprecated)]
                    label: Some(label),
                    style: TextInputStyle::Paragraph,
                    min_length: Some(1),
                    max_length: Some(500),
                    placeholder: None,
                    required: Some(true),
                    value: None,
                })],
            })]),
            ..Default::default()
        }),
    }
}

pub async fn start_rental(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    voice_channel_id: Option<Id<ChannelMarker>>,
    lang: &str,
) -> BotResult<StartRentalResult> {
    let existing = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            rental_facade::find_active_session_for_user(txn, guild_id.get(), user_id.get()).await
        })
    })
    .await?;

    if let Some(existing) = existing {
        let has_pending_state = state.rental_states.iter().any(|entry| {
            matches!(
                &entry.state,
                RentalState::AwaitingPurpose {
                    session_id,
                    host_user_id,
                    ..
                } if *session_id == existing.id && *host_user_id == user_id.get()
            )
        });
        if existing.state == rental_sessions::STATE_AWAITING_PURPOSE && has_pending_state {
            return Ok(StartRentalResult::Started {
                session_id: existing.id,
                room_id: existing.room_id,
                response: build_purpose_modal(&state, lang, existing.id, existing.room_id),
            });
        }

        return Ok(StartRentalResult::AlreadyRenting);
    }

    // Prefer the VC's registered room if one exists
    let room = if let Some(vc_id) = voice_channel_id {
        let vc_room = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
            Box::pin(async move {
                room_facade::find_room_by_voice_channel(txn, guild_id.get(), vc_id.get()).await
            })
        })
        .await?;
        if let Some(room) = vc_room {
            let active_session = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
                let room_id = room.id;
                Box::pin(
                    async move { rental_facade::find_active_session_for_room(txn, room_id).await },
                )
            })
            .await?;
            if active_session.is_none() {
                Some(room)
            } else {
                None
            }
        } else {
            with_guild_context(&state.db.guild, guild_id.get(), |txn| {
                Box::pin(async move { room_facade::find_available_room(txn, guild_id.get()).await })
            })
            .await?
        }
    } else {
        with_guild_context(&state.db.guild, guild_id.get(), |txn| {
            Box::pin(async move { room_facade::find_available_room(txn, guild_id.get()).await })
        })
        .await?
    };

    let Some(room) = room else {
        return Ok(StartRentalResult::NoAvailableRooms);
    };

    let room_id = room.id;
    let vc_channel_for_key = voice_channel_id
        .or_else(|| room.voice_channel_id.map(|id| Id::new(id as u64)))
        .unwrap_or_else(|| Id::new(0));

    let session = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            rental_facade::create_session(txn, guild_id.get(), room_id, user_id.get()).await
        })
    })
    .await?;

    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move { room_facade::set_room_availability(txn, room_id, false).await })
    })
    .await?;

    let timeout = spawn_purpose_timeout(
        state.clone(),
        guild_id.get(),
        vc_channel_for_key.get(),
        session.id,
        0,
        Duration::from_secs(rental_facade::PURPOSE_TIMEOUT_MINUTES as u64 * 60),
    );

    let key = (guild_id.get(), vc_channel_for_key.get());
    state.rental_states.insert(
        key,
        RentalStateEntry {
            state: RentalState::AwaitingPurpose {
                session_id: session.id,
                host_user_id: user_id.get(),
                timeout_task: timeout,
            },
            room_id,
        },
    );

    let response = build_purpose_modal(&state, lang, session.id, room_id);
    Ok(StartRentalResult::Started {
        session_id: session.id,
        room_id,
        response,
    })
}

pub async fn submit_purpose(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    session_id: i32,
    purpose: String,
    voice_channel_id: u64,
    lang: &str,
) -> BotResult<String> {
    let key = (guild_id.get(), voice_channel_id);
    if let Some(entry) = state.rental_states.get(&key) {
        entry.abort_timeout();
    }

    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        let purpose_clone = purpose.clone();
        Box::pin(async move {
            rental_facade::set_purpose(txn, session_id, purpose_clone).await?;
            Ok(())
        })
    })
    .await?;

    let session = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            rental_sessions::Entity::find_by_id(session_id)
                .one(txn)
                .await
                .map_err(crate::error::BotError::from)?
                .ok_or_else(|| crate::error::BotError::NotFound(format!("session {session_id}")))
        })
    })
    .await?;

    let room = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        let room_id = session.room_id;
        Box::pin(async move {
            rooms::Entity::find_by_id(room_id)
                .one(txn)
                .await
                .map_err(crate::error::BotError::from)?
                .ok_or_else(|| crate::error::BotError::NotFound(format!("room {room_id}")))
        })
    })
    .await?;

    if let Some(mut entry) = state.rental_states.get_mut(&key) {
        entry.state = RentalState::Active {
            session_id,
            host_user_id: user_id.get(),
        };
    }

    let channel_mention = room
        .text_channel_id
        .map(|id| format!("<#{id}>"))
        .or_else(|| room.voice_channel_id.map(|id| format!("<#{id}>")))
        .unwrap_or_else(|| "the room".to_string());

    let mut args = FluentArgs::new();
    args.set("channel", channel_mention);
    Ok(state
        .i18n
        .get_with_args(lang, &MessageKey::BotRentalAssigned, Some(&args)))
}

pub async fn release_rental(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    _user_id: Id<UserMarker>,
    voice_channel_id: u64,
) -> BotResult<String> {
    let lang = resolve_language(&state, guild_id, None).await;

    let key = (guild_id.get(), voice_channel_id);
    let (session_id, room_id) = {
        let entry = state.rental_states.get(&key);
        let Some(entry) = entry else {
            return Ok(state.i18n.get(&lang, &MessageKey::ErrorGeneric));
        };
        entry.abort_timeout();
        (entry.session_id(), entry.room_id)
    };

    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            rental_facade::release_session(txn, session_id).await?;
            room_facade::set_room_availability(txn, room_id, true).await
        })
    })
    .await?;

    state.rental_states.remove(&key);
    Ok(state.i18n.get(&lang, &MessageKey::BotRentalReleased))
}
