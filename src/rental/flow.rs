use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::entities::{rental_sessions, rooms};
use crate::error::BotResult;
use crate::facade::{
    question_preset as question_preset_facade, rental as rental_facade, room as room_facade,
};
use crate::facade::question_preset::{QuestionInput, QuestionWithInput};
use crate::i18n::MessageKey;
use crate::language::resolve_language;
use crate::rental::state_machine::{RentalState, RentalStateEntry, get_dropdown_answers};
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
    channel::message::{
        MessageFlags,
        component::{
            ActionRow, Button, ButtonStyle, Component, SelectMenu, SelectMenuOption,
            SelectMenuType, TextInput, TextInputStyle,
        },
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

pub fn build_purpose_modal(
    state: &AppState,
    lang: &str,
    session_id: i32,
    room_id: i32,
    questions: &[String],
) -> InteractionResponse {
    let title = state.i18n.get(lang, &MessageKey::BotRentalRequestStart);
    let label = if questions.is_empty() {
        state.i18n.get(lang, &MessageKey::BotRentalPurposeLabel)
    } else {
        state.i18n.get(lang, &MessageKey::BotRentalAnswersLabel)
    };
    let value = if questions.is_empty() {
        None
    } else {
        let answer_prefix = state.i18n.get(lang, &MessageKey::BotRentalAnswerPrefix);
        Some(answer_template(questions, &answer_prefix))
    };

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
                    max_length: Some(4000),
                    placeholder: None,
                    required: Some(true),
                    value,
                })],
            })]),
            ..Default::default()
        }),
    }
}

/// Build a modal with individual TextInputs for text-only questions (used after dropdown phase).
pub fn build_text_questions_modal(
    state: &AppState,
    lang: &str,
    session_id: i32,
    room_id: i32,
    text_questions: &[&QuestionWithInput],
) -> InteractionResponse {
    let title = state.i18n.get(lang, &MessageKey::BotRentalRequestStart);

    let components: Vec<Component> = text_questions
        .iter()
        .take(5) // Discord modal limit: 5 ActionRows
        .map(|q| {
            Component::ActionRow(ActionRow {
                id: None,
                components: vec![Component::TextInput(TextInput {
                    id: None,
                    custom_id: format!("qt_{}", q.index),
                    #[allow(deprecated)]
                    label: Some(format!("{}. {}", q.index + 1, q.text)),
                    style: TextInputStyle::Short,
                    min_length: Some(1),
                    max_length: Some(200),
                    placeholder: None,
                    required: Some(true),
                    value: None,
                })],
            })
        })
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

/// Build an ephemeral message with select menus for dropdown questions + a confirm button.
/// Up to 4 dropdown questions can be shown (Discord limit: 5 ActionRows, 1 used for button).
pub fn build_dropdown_selection_message(
    state: &AppState,
    lang: &str,
    session_id: i32,
    room_id: i32,
    dropdown_questions: &[&QuestionWithInput],
    existing_answers: &[Option<String>],
) -> InteractionResponse {
    let confirm_label = state.i18n.get(lang, &MessageKey::BotRentalDropdownConfirm);
    let prompt = state.i18n.get(lang, &MessageKey::BotRentalDropdownPrompt);

    let mut action_rows: Vec<Component> = dropdown_questions
        .iter()
        .take(4)
        .map(|q| {
            let options = if let QuestionInput::Dropdown(opts) = &q.input {
                let prev = existing_answers
                    .get(q.index)
                    .and_then(|a| a.as_deref())
                    .unwrap_or("");
                opts.iter()
                    .map(|opt| SelectMenuOption {
                        label: opt.clone(),
                        value: opt.clone(),
                        description: None,
                        emoji: None,
                        default: opt == prev,
                    })
                    .collect::<Vec<_>>()
            } else {
                vec![]
            };

            Component::ActionRow(ActionRow {
                id: None,
                components: vec![Component::SelectMenu(SelectMenu {
                    id: None,
                    channel_types: None,
                    custom_id: format!("dqa:{session_id}:{}", q.index),
                    default_values: None,
                    disabled: false,
                    kind: SelectMenuType::Text,
                    max_values: Some(1),
                    min_values: Some(1),
                    options: Some(options),
                    placeholder: Some(format!("{}. {}", q.index + 1, q.text)),
                    required: None,
                })],
            })
        })
        .collect();

    action_rows.push(Component::ActionRow(ActionRow {
        id: None,
        components: vec![Component::Button(Button {
            id: None,
            custom_id: Some(format!("dqc:{session_id}:{room_id}")),
            disabled: false,
            emoji: None,
            label: Some(confirm_label),
            style: ButtonStyle::Primary,
            url: None,
            sku_id: None,
        })],
    }));

    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some(prompt),
            components: Some(action_rows),
            flags: Some(MessageFlags::EPHEMERAL),
            ..Default::default()
        }),
    }
}

pub async fn build_purpose_modal_for_room(
    state: &AppState,
    guild_id: Id<GuildMarker>,
    lang: &str,
    session_id: i32,
    room_id: i32,
) -> BotResult<InteractionResponse> {
    let questions_with_inputs =
        questions_with_inputs_for_room(state, guild_id, room_id).await?;
    let dropdown_qs: Vec<&QuestionWithInput> = questions_with_inputs
        .iter()
        .filter(|q| matches!(q.input, QuestionInput::Dropdown(_)))
        .collect();

    if !dropdown_qs.is_empty() {
        let existing = get_dropdown_answers(&state.rental_states, session_id);
        Ok(build_dropdown_selection_message(
            state,
            lang,
            session_id,
            room_id,
            &dropdown_qs,
            &existing,
        ))
    } else {
        let simple_questions: Vec<String> =
            questions_with_inputs.iter().map(|q| q.text.clone()).collect();
        Ok(build_purpose_modal(
            state,
            lang,
            session_id,
            room_id,
            &simple_questions,
        ))
    }
}

fn answer_template(questions: &[String], answer_prefix: &str) -> String {
    questions
        .iter()
        .enumerate()
        .map(|(index, question)| format!("{}. {question}\n{answer_prefix}: ", index + 1))
        .collect::<Vec<_>>()
        .join("\n\n")
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
            let dropdown_qs: Vec<&QuestionWithInput> = questions_with_inputs
                .iter()
                .filter(|q| matches!(q.input, QuestionInput::Dropdown(_)))
                .collect();

            let response = if !dropdown_qs.is_empty() {
                let existing_answers =
                    get_dropdown_answers(&state.rental_states, existing.id);
                build_dropdown_selection_message(
                    &state,
                    lang,
                    existing.id,
                    existing.room_id,
                    &dropdown_qs,
                    &existing_answers,
                )
            } else {
                let simple_questions: Vec<String> =
                    questions_with_inputs.iter().map(|q| q.text.clone()).collect();
                build_purpose_modal(&state, lang, existing.id, existing.room_id, &simple_questions)
            };

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
    let questions_with_inputs =
        questions_with_inputs_for_room(&state, guild_id, room_id).await?;
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
                dropdown_answers: vec![None; 10],
            },
            room_id,
        },
    );

    let dropdown_qs: Vec<&QuestionWithInput> = questions_with_inputs
        .iter()
        .filter(|q| matches!(q.input, QuestionInput::Dropdown(_)))
        .collect();

    let response = if !dropdown_qs.is_empty() {
        build_dropdown_selection_message(
            &state,
            lang,
            session.id,
            room_id,
            &dropdown_qs,
            &vec![None; 10],
        )
    } else {
        let simple_questions: Vec<String> =
            questions_with_inputs.iter().map(|q| q.text.clone()).collect();
        build_purpose_modal(&state, lang, session.id, room_id, &simple_questions)
    };

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
    question_preset_facade::assemble_purpose(questions, dropdown_answers, text_answers, answer_prefix)
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
