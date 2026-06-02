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
use crate::rental::state_machine::{RentalPromptMessage, RentalState, RentalStateEntry};
use crate::rental::timeout::spawn_purpose_timeout;
use fluent_bundle::FluentArgs;
use sea_orm::EntityTrait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker, MessageMarker, UserMarker},
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

/// Custom-id used for the system VC selection question inside the unified modal.
/// Kept distinct from preset-question custom ids (`mq_{index}`) so the submit handler
/// can recognise the VC answer without colliding with any preset question.
pub const MODAL_VC_CUSTOM_ID: &str = "mq_vc";

/// Custom-id prefix for a question's input inside the unified modal: `mq_{index}`.
fn modal_question_custom_id(index: usize) -> String {
    format!("mq_{index}")
}

/// Optional system "which VC?" question prepended to the unified modal.
pub struct VcChoiceSpec {
    /// (room_id, label) pairs, listed in the dropdown in this order.
    pub options: Vec<(i32, String)>,
    /// Pre-allocated room shown as the default selection.
    pub default_room_id: i32,
    /// Localised question label.
    pub label: String,
}

/// Truncate a string to `max` characters on a char boundary (Discord label/option limits
/// are measured in characters; naive byte slicing would panic on multi-byte text).
fn truncate_chars(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

/// Build the single unified modal that collects every question's answer in one submission:
/// dropdown questions become `Label`-wrapped select menus, free-text questions become
/// `Label`-wrapped text inputs. Each input's `Label` carries the question text.
///
/// When `vc_choice` is `Some`, a "which VC?" dropdown is prepended as the first component.
/// Because Discord caps modals at 5 top-level components, preset questions past that limit
/// are dropped — the VC question is required so it always wins the slot.
pub fn build_unified_modal(
    state: &AppState,
    lang: &str,
    session_id: i32,
    room_id: i32,
    questions: &[QuestionWithInput],
    vc_choice: Option<&VcChoiceSpec>,
) -> InteractionResponse {
    let title = state.i18n.get(lang, &MessageKey::BotRentalRequestStart);

    let mut components: Vec<Component> = Vec::new();
    if let Some(spec) = vc_choice {
        components.push(build_vc_choice_label(spec));
    }
    let remaining = MODAL_MAX_QUESTIONS.saturating_sub(components.len());
    components.extend(questions.iter().take(remaining).map(build_question_label));

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

fn build_vc_choice_label(spec: &VcChoiceSpec) -> Component {
    let options = spec
        .options
        .iter()
        .map(|(room_id, label)| SelectMenuOption {
            label: truncate_chars(label, 100),
            value: room_id.to_string(),
            description: None,
            emoji: None,
            default: *room_id == spec.default_room_id,
        })
        .collect::<Vec<_>>();

    let select = Component::SelectMenu(SelectMenu {
        id: None,
        channel_types: None,
        custom_id: MODAL_VC_CUSTOM_ID.to_string(),
        default_values: None,
        disabled: false,
        kind: SelectMenuType::Text,
        max_values: Some(1),
        min_values: Some(1),
        options: Some(options),
        placeholder: None,
        required: Some(true),
    });

    Component::Label(Label {
        id: None,
        label: truncate_chars(&spec.label, 45),
        description: None,
        component: Box::new(select),
    })
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
///
/// If `include_vc_question` is true, the system VC dropdown is added as the first
/// component. Pass false when the user is already inside the rental VC (the choice is
/// obvious in that case).
pub async fn build_unified_modal_for_room(
    state: &AppState,
    guild_id: Id<GuildMarker>,
    lang: &str,
    session_id: i32,
    room_id: i32,
    include_vc_question: bool,
) -> BotResult<InteractionResponse> {
    let questions = questions_with_inputs_for_room(state, guild_id, room_id).await?;
    let vc_choice = if include_vc_question {
        build_vc_choice_spec(state, guild_id, lang, room_id).await?
    } else {
        None
    };
    Ok(build_unified_modal(
        state,
        lang,
        session_id,
        room_id,
        &questions,
        vc_choice.as_ref(),
    ))
}

/// Build the VC choice spec listing every currently-available rental room in the guild,
/// guaranteeing the pre-allocated room (`current_room_id`) appears even if its in-memory
/// `is_available` flag has already been flipped to false by `start_rental`.
///
/// Returns `None` when no rooms are eligible — the caller falls back to a modal without
/// the VC question, which preserves the previous flow rather than blocking the user.
pub(crate) async fn build_vc_choice_spec(
    state: &AppState,
    guild_id: Id<GuildMarker>,
    lang: &str,
    current_room_id: i32,
) -> BotResult<Option<VcChoiceSpec>> {
    let label = state.i18n.get(lang, &MessageKey::BotRentalVcQuestion);

    let available = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move { room_facade::list_available_rooms(txn, guild_id.get()).await })
    })
    .await?;

    let mut options: Vec<(i32, String)> = Vec::with_capacity(available.len());
    for room in available {
        let label = room_choice_label(state, &room).await;
        options.push((room.id, label));
    }

    if !options.iter().any(|(id, _)| *id == current_room_id)
        && let Some(current) = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
            Box::pin(async move { room_facade::find_room_by_id(txn, current_room_id).await })
        })
        .await?
    {
        let label = room_choice_label(state, &current).await;
        options.insert(0, (current.id, label));
    }

    if options.is_empty() {
        return Ok(None);
    }

    options.truncate(25);

    Ok(Some(VcChoiceSpec {
        options,
        default_room_id: current_room_id,
        label,
    }))
}

async fn room_choice_label(state: &AppState, room: &rooms::Model) -> String {
    if let Some(vc) = room.voice_channel_id {
        return fetch_channel_name(state, vc as u64)
            .await
            .unwrap_or_else(|| format!("#{}", vc));
    }
    if let Some(tc) = room.text_channel_id {
        return fetch_channel_name(state, tc as u64)
            .await
            .unwrap_or_else(|| format!("#{}", tc));
    }
    format!("Room {}", room.id)
}

async fn fetch_channel_name(state: &AppState, channel_id: u64) -> Option<String> {
    let response = state.http.channel(Id::new(channel_id)).await.ok()?;
    let channel = response.model().await.ok()?;
    channel.name
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
            let include_vc_question =
                !user_is_in_room(&state, guild_id, user_id, existing.room_id).await?;
            let vc_choice = if include_vc_question {
                build_vc_choice_spec(&state, guild_id, lang, existing.room_id).await?
            } else {
                None
            };
            let response = build_unified_modal(
                &state,
                lang,
                existing.id,
                existing.room_id,
                &questions_with_inputs,
                vc_choice.as_ref(),
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
                prompt_message: None,
            },
            room_id,
        },
    );

    let include_vc_question = !user_is_in_room(&state, guild_id, user_id, room_id).await?;
    let vc_choice = if include_vc_question {
        build_vc_choice_spec(&state, guild_id, lang, room_id).await?
    } else {
        None
    };
    let response = build_unified_modal(
        &state,
        lang,
        session.id,
        room_id,
        &questions_with_inputs,
        vc_choice.as_ref(),
    );

    Ok(StartRentalResult::AwaitingQuestions {
        session_id: session.id,
        room_id,
        response,
    })
}

/// Outcome of attempting to swap a pending rental's room based on the modal's VC selection.
enum SwapOutcome {
    /// The selection matched the already-allocated room — nothing to do.
    NoChange,
    /// The selected room is occupied by another active session.
    Conflict,
    /// Swap succeeded; the state map's key has moved to this new (guild, vc) tuple.
    Moved((u64, u64)),
}

/// Swap the pending session to `target_room_id` if it differs from the session's current
/// room. Updates room availability, the session's `room_id`, and the in-memory state map.
async fn swap_room_if_changed(
    state: &AppState,
    guild_id: Id<GuildMarker>,
    session_id: i32,
    target_room_id: i32,
    current_key: (u64, u64),
) -> BotResult<SwapOutcome> {
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

    if session.room_id == target_room_id {
        return Ok(SwapOutcome::NoChange);
    }

    let old_room_id = session.room_id;

    let conflict = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(
            async move { rental_facade::find_active_session_for_room(txn, target_room_id).await },
        )
    })
    .await?;
    if conflict.is_some() {
        return Ok(SwapOutcome::Conflict);
    }

    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            room_facade::set_room_availability(txn, old_room_id, true).await?;
            room_facade::set_room_availability(txn, target_room_id, false).await?;
            rental_facade::set_session_room(txn, session_id, target_room_id).await?;
            Ok::<_, crate::error::BotError>(())
        })
    })
    .await?;

    let target_room = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move { room_facade::find_room_by_id(txn, target_room_id).await })
    })
    .await?
    .ok_or_else(|| crate::error::BotError::NotFound(format!("room {target_room_id}")))?;

    let new_vc_for_key = target_room
        .voice_channel_id
        .map(|id| id as u64)
        .unwrap_or(0);
    let new_key = (guild_id.get(), new_vc_for_key);

    if new_key != current_key {
        if let Some((_, mut entry)) = state.rental_states.remove(&current_key) {
            entry.room_id = target_room_id;
            state.rental_states.insert(new_key, entry);
        }
    } else if let Some(mut entry) = state.rental_states.get_mut(&new_key) {
        entry.room_id = target_room_id;
    }

    Ok(SwapOutcome::Moved(new_key))
}

/// True when the user's currently-tracked voice channel equals the room's voice channel.
pub(crate) async fn user_is_in_room(
    state: &AppState,
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    room_id: i32,
) -> BotResult<bool> {
    let room = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move { room_facade::find_room_by_id(txn, room_id).await })
    })
    .await?;
    let Some(room) = room else {
        return Ok(false);
    };
    let Some(room_vc) = room.voice_channel_id else {
        return Ok(false);
    };
    Ok(state
        .voice_occupancy
        .channel_for_user(guild_id.get(), user_id.get())
        == Some(room_vc as u64))
}

pub async fn submit_purpose(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    user_id: Id<UserMarker>,
    session_id: i32,
    purpose: String,
    voice_channel_id: u64,
    lang: &str,
    selected_room_id: Option<i32>,
) -> BotResult<String> {
    let mut key = (guild_id.get(), voice_channel_id);

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

    // Honour the VC dropdown: if the user picked a different room, swap before activating.
    if let Some(target_room_id) = selected_room_id {
        match swap_room_if_changed(&state, guild_id, session_id, target_room_id, key).await? {
            SwapOutcome::NoChange => {}
            SwapOutcome::Conflict => {
                return Ok(state.i18n.get(lang, &MessageKey::BotRentalVcRoomOccupied));
            }
            SwapOutcome::Moved(new_key) => {
                key = new_key;
            }
        }
    }

    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        let purpose_clone = purpose.clone();
        Box::pin(async move { rental_facade::set_purpose(txn, session_id, purpose_clone).await })
    })
    .await?;

    // `scheduled_tasks` lives in the `worker` schema where the guild role only has
    // SELECT/INSERT. UPDATE is the system role's job (see CLAUDE.md / grant_permissions).
    rental_facade::mark_session_tasks_processed(&state.db.system, session_id).await?;

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

    delete_purpose_prompt_message(&state, guild_id.get(), key.1, session_id).await;

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

pub fn attach_purpose_prompt_message(
    state: &AppState,
    guild_id: Id<GuildMarker>,
    voice_channel_id: Id<ChannelMarker>,
    session_id: i32,
    prompt_message: RentalPromptMessage,
) {
    let key = (guild_id.get(), voice_channel_id.get());
    if let Some(mut entry) = state.rental_states.get_mut(&key)
        && let RentalState::AwaitingPurpose {
            session_id: pending_session_id,
            prompt_message: stored_message,
            ..
        } = &mut entry.state
        && *pending_session_id == session_id
    {
        *stored_message = Some(prompt_message);
    }
}

pub(crate) async fn delete_purpose_prompt_message(
    state: &AppState,
    guild_id: u64,
    voice_channel_id: u64,
    session_id: i32,
) {
    let prompt_message = state
        .rental_states
        .get(&(guild_id, voice_channel_id))
        .and_then(|entry| match &entry.state {
            RentalState::AwaitingPurpose {
                session_id: pending_session_id,
                prompt_message,
                ..
            } if *pending_session_id == session_id => *prompt_message,
            _ => None,
        });

    if let Some(prompt_message) = prompt_message {
        let result = state
            .http
            .delete_message(
                Id::<ChannelMarker>::new(prompt_message.channel_id),
                Id::<MessageMarker>::new(prompt_message.message_id),
            )
            .await;
        if let Err(err) = result {
            tracing::warn!(
                guild_id,
                voice_channel_id,
                session_id,
                channel_id = prompt_message.channel_id,
                message_id = prompt_message.message_id,
                error = %err,
                "Failed to delete rental purpose prompt message"
            );
        }
    }
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

    // Retire the matching scheduled_tasks row so the timer can never resurrect after a
    // restart (`restore_pending_timeouts`). Without this, a cancelled rental's DB-side
    // task would re-spawn on the next boot and may race with a newer rental.
    rental_facade::mark_session_tasks_processed(&state.db.system, session_id).await?;

    delete_purpose_prompt_message(&state, guild_id.get(), voice_channel_id, session_id).await;

    state.rental_states.remove(&key);

    crate::rental::status::trigger(&state, guild_id.get());

    Ok(state.i18n.get(&lang, &MessageKey::BotRentalReleased))
}
