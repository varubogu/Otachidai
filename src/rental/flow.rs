use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::entities::{rental_sessions, rooms};
use crate::error::BotResult;
use crate::facade::question_preset::{QuestionInput, QuestionWithInput};
use crate::facade::{
    question_preset as question_preset_facade, rental as rental_facade, room as room_facade,
};
use crate::i18n::MessageKey;
use crate::language::resolve_language;
use crate::rental::state_machine::{RentalState, RentalStateEntry};
use crate::rental::timeout::spawn_purpose_timeout;
use fluent_bundle::FluentArgs;
use sea_orm::EntityTrait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker, UserMarker},
};
use twilight_model::{
    channel::message::component::{
        ActionRow, Button, ButtonStyle, Component, Label, SelectMenu, SelectMenuOption,
        SelectMenuType, TextInput, TextInputStyle,
    },
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
};

pub enum StartRentalResult {
    AwaitingQuestions {
        session_id: i32,
        room_id: i32,
        response: InteractionResponse,
    },
    Assigned {
        session_id: i32,
        room_id: i32,
        message: String,
    },
    AlreadyRenting,
    NoAvailableRooms,
}

/// Discord caps a modal at 5 top-level components, so at most 5 questions fit.
const MODAL_MAX_QUESTIONS: usize = 5;

/// Custom-id prefix for a question's input inside the unified modal: `mq_{index}`.
fn modal_question_custom_id(index: usize) -> String {
    format!("mq_{index}")
}

/// Truncate a string to `max` characters on a char boundary (Discord label/option limits
/// are measured in characters; naive byte slicing would panic on multi-byte text).
fn truncate_chars(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

/// Build the single unified modal that collects every question's answer in one submission:
/// dropdown questions become `Label`-wrapped select menus, free-text questions become
/// `Label`-wrapped text inputs. Each input's `Label` carries the question text.
pub fn build_unified_modal(
    state: &AppState,
    lang: &str,
    session_id: i32,
    room_id: i32,
    questions: &[QuestionWithInput],
) -> InteractionResponse {
    let title = state.i18n.get(lang, &MessageKey::BotRentalRequestStart);

    let components: Vec<Component> = questions
        .iter()
        .take(MODAL_MAX_QUESTIONS)
        .map(build_question_label)
        .collect();

    InteractionResponse {
        kind: InteractionResponseType::Modal,
        data: Some(InteractionResponseData {
            custom_id: Some(format!("purpose_modal:{session_id}:{room_id}")),
            title: Some(title),
            components: Some(components),
            ..Default::default()
        }),
    }
}

/// Wrap one question's input component in a `Label` carrying its (truncated) question text.
fn build_question_label(q: &QuestionWithInput) -> Component {
    let custom_id = modal_question_custom_id(q.index);
    let label_text = truncate_chars(&format!("{}. {}", q.index + 1, q.text), 45);

    let inner = match &q.input {
        QuestionInput::Dropdown(opts) => {
            let options = opts
                .iter()
                .map(|opt| SelectMenuOption {
                    label: truncate_chars(opt, 100),
                    value: truncate_chars(opt, 100),
                    description: None,
                    emoji: None,
                    default: false,
                })
                .collect::<Vec<_>>();
            Component::SelectMenu(SelectMenu {
                id: None,
                channel_types: None,
                custom_id,
                default_values: None,
                disabled: false,
                kind: SelectMenuType::Text,
                max_values: Some(1),
                min_values: Some(1),
                options: Some(options),
                placeholder: None,
                required: Some(true),
            })
        }
        QuestionInput::Text => Component::TextInput(TextInput {
            id: None,
            custom_id,
            // The wrapping `Label` supplies the visible label in modals; the deprecated
            // per-input label must stay empty to avoid a duplicate.
            #[allow(deprecated)]
            label: None,
            style: TextInputStyle::Paragraph,
            min_length: Some(1),
            max_length: Some(1000),
            placeholder: None,
            required: Some(true),
            value: None,
        }),
    };

    Component::Label(Label {
        id: None,
        label: label_text,
        description: None,
        component: Box::new(inner),
    })
}

/// Build the unified modal for a room, fetching its question preset first.
pub async fn build_unified_modal_for_room(
    state: &AppState,
    guild_id: Id<GuildMarker>,
    lang: &str,
    session_id: i32,
    room_id: i32,
) -> BotResult<InteractionResponse> {
    let questions = questions_with_inputs_for_room(state, guild_id, room_id).await?;
    Ok(build_unified_modal(
        state, lang, session_id, room_id, &questions,
    ))
}

/// Build the content + components for the message posted when a user joins a rental VC:
/// a single "answer" button that opens the unified question modal. A button is required
/// because modals can only be opened from an interaction, not a gateway event.
pub fn build_join_answer_button(
    state: &AppState,
    lang: &str,
    user_id: Id<UserMarker>,
    session_id: i32,
    room_id: i32,
) -> (String, Vec<Component>) {
    let prompt = state.i18n.get(lang, &MessageKey::BotRentalRequestStart);
    let button_label = state.i18n.get(lang, &MessageKey::RentAnswerButtonLabel);
    let content = format!("<@{}>\n{}", user_id.get(), prompt);

    let components = vec![Component::ActionRow(ActionRow {
        id: None,
        components: vec![Component::Button(Button {
            id: None,
            custom_id: Some(format!("answer:{session_id}:{room_id}")),
            disabled: false,
            emoji: None,
            label: Some(button_label),
            style: ButtonStyle::Primary,
            url: None,
            sku_id: None,
        })],
    })];

    (content, components)
}

pub(crate) async fn questions_with_inputs_for_room(
    state: &AppState,
    guild_id: Id<GuildMarker>,
    room_id: i32,
) -> BotResult<Vec<QuestionWithInput>> {
    let room = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            rooms::Entity::find_by_id(room_id)
                .one(txn)
                .await
                .map_err(crate::error::BotError::from)?
                .ok_or_else(|| crate::error::BotError::NotFound(format!("room {room_id}")))
        })
    })
    .await?;

    let Some(preset_id) = room.question_preset_id else {
        return Ok(Vec::new());
    };

    let preset = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move { question_preset_facade::find_by_id(txn, preset_id).await })
    })
    .await?;

    Ok(preset
        .map(|p| question_preset_facade::model_questions_with_inputs(&p))
        .unwrap_or_default())
}

fn assigned_message(state: &AppState, lang: &str, room: &rooms::Model) -> String {
    let channel_mention = room_channel_mention(room);
    let mut args = FluentArgs::new();
    args.set("channel", channel_mention);
    state
        .i18n
        .get_with_args(lang, &MessageKey::BotRentalAssigned, Some(&args))
}

fn room_channel_mention(room: &rooms::Model) -> String {
    room.text_channel_id
        .map(|id| format!("<#{id}>"))
        .or_else(|| room.voice_channel_id.map(|id| format!("<#{id}>")))
        .unwrap_or_else(|| "the room".to_string())
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
            let questions_with_inputs =
                questions_with_inputs_for_room(&state, guild_id, existing.room_id).await?;
            let response = build_unified_modal(
                &state,
                lang,
                existing.id,
                existing.room_id,
                &questions_with_inputs,
            );

            return Ok(StartRentalResult::AwaitingQuestions {
                session_id: existing.id,
                room_id: existing.room_id,
                response,
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
    let questions_with_inputs = questions_with_inputs_for_room(&state, guild_id, room_id).await?;
    let has_questions = !questions_with_inputs.is_empty();

    let session = if has_questions {
        with_guild_context(&state.db.guild, guild_id.get(), |txn| {
            Box::pin(async move {
                rental_facade::create_session(txn, guild_id.get(), room_id, user_id.get()).await
            })
        })
        .await?
    } else {
        with_guild_context(&state.db.guild, guild_id.get(), |txn| {
            Box::pin(async move {
                rental_facade::create_active_session(txn, guild_id.get(), room_id, user_id.get())
                    .await
            })
        })
        .await?
    };

    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move { room_facade::set_room_availability(txn, room_id, false).await })
    })
    .await?;

    crate::rental::status::trigger(&state, guild_id.get());

    let key = (guild_id.get(), vc_channel_for_key.get());

    if !has_questions {
        state.rental_states.insert(
            key,
            RentalStateEntry {
                state: RentalState::Active {
                    session_id: session.id,
                    host_user_id: user_id.get(),
                },
                room_id,
            },
        );

        return Ok(StartRentalResult::Assigned {
            session_id: session.id,
            room_id,
            message: assigned_message(&state, lang, &room),
        });
    }

    let timeout = spawn_purpose_timeout(
        state.clone(),
        guild_id.get(),
        vc_channel_for_key.get(),
        session.id,
        0,
        Duration::from_secs(rental_facade::PURPOSE_TIMEOUT_MINUTES as u64 * 60),
    );

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

    let response = build_unified_modal(&state, lang, session.id, room_id, &questions_with_inputs);

    Ok(StartRentalResult::AwaitingQuestions {
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

    // Guard against resurrecting an expired session: if the in-memory AwaitingPurpose
    // entry is gone (purpose timeout fired, or the user left the VC and the rental was
    // released), a late modal submit must NOT write the session back to Active.
    let is_pending = match state.rental_states.get(&key) {
        Some(entry) => matches!(
            &entry.state,
            RentalState::AwaitingPurpose { session_id: sid, .. } if *sid == session_id
        ),
        None => false,
    };
    if !is_pending {
        return Ok(state.i18n.get(lang, &MessageKey::BotRentalExpired));
    }

    if let Some(entry) = state.rental_states.get(&key) {
        entry.abort_timeout();
    }

    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        let purpose_clone = purpose.clone();
        Box::pin(async move {
            rental_facade::set_purpose(txn, session_id, purpose_clone).await?;
            rental_facade::mark_session_tasks_processed(txn, session_id).await?;
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

    crate::rental::status::trigger(&state, guild_id.get());

    Ok(assigned_message(&state, lang, &room))
}

/// Assemble a structured purpose string from dropdown answers (in state) and text answers
/// (from modal). Delegates to `question_preset_facade::assemble_purpose`.
pub fn assemble_purpose_from_parts(
    questions: &[QuestionWithInput],
    dropdown_answers: &[Option<String>],
    text_answers: &HashMap<usize, String>,
    answer_prefix: &str,
) -> String {
    question_preset_facade::assemble_purpose(
        questions,
        dropdown_answers,
        text_answers,
        answer_prefix,
    )
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

    crate::rental::status::trigger(&state, guild_id.get());

    Ok(state.i18n.get(&lang, &MessageKey::BotRentalReleased))
}
