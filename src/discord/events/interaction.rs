use crate::app_state::AppState;
use crate::error::BotResult;
use crate::facade::question_preset::QuestionInput;
use crate::i18n::MessageKey;
use crate::language::resolve_language;
use crate::rental::state_machine::{find_vc_for_session, get_dropdown_answers};
use crate::rental::{flow as rental_flow, handoff, state_machine::RentalState};
use std::collections::HashMap;
use std::sync::Arc;
use twilight_model::{
    application::interaction::{
        Interaction, InteractionData, InteractionType, modal::ModalInteractionComponent,
    },
    channel::message::MessageFlags,
    gateway::payload::incoming::InteractionCreate,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{
        Id,
        marker::{GuildMarker, UserMarker},
    },
};

pub async fn handle(state: Arc<AppState>, event: InteractionCreate) -> BotResult<()> {
    let interaction = event.0;
    let guild_id = match interaction.guild_id {
        Some(g) => g,
        None => return Ok(()),
    };

    match interaction.kind {
        InteractionType::ApplicationCommand => handle_command(state, guild_id, interaction).await,
        InteractionType::MessageComponent => handle_component(state, guild_id, interaction).await,
        InteractionType::ModalSubmit => handle_modal(state, guild_id, interaction).await,
        _ => Ok(()),
    }
}

async fn handle_command(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    interaction: Interaction,
) -> BotResult<()> {
    let Some(InteractionData::ApplicationCommand(ref data)) = interaction.data else {
        return Ok(());
    };
    let user_id = interaction_user_id(&interaction);

    let is_admin_command = matches!(
        data.name.as_str(),
        "register_report_channel"
            | "register_rental_button_channel"
            | "register_question_preset"
            | "register_room"
            | "delete_room"
            | "register_group"
            | "delete_group"
            | "set_room_group"
    );

    if is_admin_command && !check_admin_permission(&interaction) {
        let lang = get_lang(&state, guild_id, interaction.locale.as_deref()).await;
        let msg = state.i18n.get(&lang, &MessageKey::AdminPermissionDenied);
        respond_ephemeral(&state, &interaction, &msg).await?;
        return Ok(());
    }

    let lang = get_lang(&state, guild_id, interaction.locale.as_deref()).await;

    let response: InteractionResponse = match data.name.as_str() {
        "register_report_channel" => {
            let msg = crate::discord::commands::admin::register_report_channel::handle(
                state.clone(),
                guild_id,
                data,
                &lang,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::error!("{e}");
                "Error".to_string()
            });
            simple_response(&msg)
        }
        "register_rental_button_channel" => {
            let msg = crate::discord::commands::admin::register_rental_button_channel::handle(
                state.clone(),
                guild_id,
                data,
                &lang,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::error!("{e}");
                "Error".to_string()
            });
            simple_response(&msg)
        }
        "register_question_preset" => {
            let msg = crate::discord::commands::admin::register_question_preset::handle(
                state.clone(),
                guild_id,
                data,
                &lang,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::error!("{e}");
                "Error".to_string()
            });
            simple_response(&msg)
        }
        "register_room" => {
            let msg = crate::discord::commands::admin::register_room::handle(
                state.clone(),
                guild_id,
                data,
                &lang,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::error!("{e}");
                "Error".to_string()
            });
            simple_response(&msg)
        }
        "delete_room" => {
            let msg = crate::discord::commands::admin::delete_room::handle(
                state.clone(),
                guild_id,
                data,
                &lang,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::error!("{e}");
                "Error".to_string()
            });
            simple_response(&msg)
        }
        "register_group" => {
            let msg = crate::discord::commands::admin::register_group::handle(
                state.clone(),
                guild_id,
                data,
                &lang,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::error!("{e}");
                "Error".to_string()
            });
            simple_response(&msg)
        }
        "delete_group" => {
            let msg = crate::discord::commands::admin::delete_group::handle(
                state.clone(),
                guild_id,
                data,
                &lang,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::error!("{e}");
                "Error".to_string()
            });
            simple_response(&msg)
        }
        "set_room_group" => {
            let msg = crate::discord::commands::admin::set_room_group::handle(
                state.clone(),
                guild_id,
                data,
                &lang,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::error!("{e}");
                "Error".to_string()
            });
            simple_response(&msg)
        }
        "rent" => {
            crate::discord::commands::user::rent::handle(state.clone(), guild_id, user_id, &lang)
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("{e}");
                    simple_response("Error")
                })
        }
        "help" => {
            let msg = crate::discord::commands::user::help::handle(state.clone(), &lang)
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("{e}");
                    "Error".to_string()
                });
            simple_response(&msg)
        }
        unknown => {
            tracing::warn!("Unknown command: {unknown}");
            return Ok(());
        }
    };

    state
        .http
        .interaction(state.application_id)
        .create_response(interaction.id, &interaction.token, &response)
        .await?;
    Ok(())
}

async fn handle_component(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    interaction: Interaction,
) -> BotResult<()> {
    let Some(InteractionData::MessageComponent(ref data)) = interaction.data else {
        return Ok(());
    };
    let user_id = interaction_user_id(&interaction);

    // rental_start (button)
    if data.custom_id == "rental_start" {
        let lang = get_lang(&state, guild_id, interaction.locale.as_deref()).await;
        let response =
            crate::discord::commands::user::rent::handle(state.clone(), guild_id, user_id, &lang)
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("{e}");
                    simple_response("Error")
                });
        state
            .http
            .interaction(state.application_id)
            .create_response(interaction.id, &interaction.token, &response)
            .await?;
        return Ok(());
    }

    // rental_start:{session_id}:{room_id} (handoff re-enter button)
    if let Some(rest) = data.custom_id.strip_prefix("rental_start:") {
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() == 2
            && let (Ok(session_id), Ok(room_id)) =
                (parts[0].parse::<i32>(), parts[1].parse::<i32>())
        {
            let lang = get_lang(&state, guild_id, interaction.locale.as_deref()).await;
            let response = if is_pending_rental_host(&state, session_id, user_id.get()) {
                rental_flow::build_purpose_modal_for_room(
                    &state, guild_id, &lang, session_id, room_id,
                )
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("{e}");
                    simple_response("Error")
                })
            } else {
                let msg = state.i18n.get(&lang, &MessageKey::ErrorGeneric);
                InteractionResponse {
                    kind: InteractionResponseType::ChannelMessageWithSource,
                    data: Some(InteractionResponseData {
                        content: Some(msg),
                        flags: Some(MessageFlags::EPHEMERAL),
                        ..Default::default()
                    }),
                }
            };

            state
                .http
                .interaction(state.application_id)
                .create_response(interaction.id, &interaction.token, &response)
                .await?;
        }
        return Ok(());
    }

    // dqa:{session_id}:{q_index} — dropdown answer selection
    if let Some(rest) = data.custom_id.strip_prefix("dqa:") {
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() == 2
            && let (Ok(session_id), Ok(q_index)) =
                (parts[0].parse::<i32>(), parts[1].parse::<usize>())
        {
            let selected = data.values.first().cloned().unwrap_or_default();

            let vc_id = find_vc_for_session(&state.rental_states, session_id);
            let key = (guild_id.get(), vc_id);
            if let Some(mut entry) = state.rental_states.get_mut(&key) {
                if let RentalState::AwaitingPurpose {
                    ref mut dropdown_answers,
                    ..
                } = entry.state
                {
                    if q_index < dropdown_answers.len() {
                        dropdown_answers[q_index] = Some(selected);
                    }
                }
            }

            // Acknowledge silently; the message stays as-is.
            state
                .http
                .interaction(state.application_id)
                .create_response(
                    interaction.id,
                    &interaction.token,
                    &InteractionResponse {
                        kind: InteractionResponseType::DeferredUpdateMessage,
                        data: None,
                    },
                )
                .await?;
        }
        return Ok(());
    }

    // dqc:{session_id}:{room_id} — dropdown confirm button
    if let Some(rest) = data.custom_id.strip_prefix("dqc:") {
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() == 2
            && let (Ok(session_id), Ok(room_id)) =
                (parts[0].parse::<i32>(), parts[1].parse::<i32>())
        {
            let lang = get_lang(&state, guild_id, interaction.locale.as_deref()).await;
            let vc_id = find_vc_for_session(&state.rental_states, session_id);
            let dropdown_answers = get_dropdown_answers(&state.rental_states, session_id);

            let questions_with_inputs =
                rental_flow::questions_with_inputs_for_room(&state, guild_id, room_id)
                    .await
                    .unwrap_or_default();

            let text_qs: Vec<&crate::facade::question_preset::QuestionWithInput> =
                questions_with_inputs
                    .iter()
                    .filter(|q| matches!(q.input, QuestionInput::Text))
                    .collect();

            let response = if text_qs.is_empty() {
                // No text questions: assemble purpose from dropdown answers only and assign.
                let answer_prefix = state.i18n.get(&lang, &MessageKey::BotRentalAnswerPrefix);
                let purpose = rental_flow::assemble_purpose_from_parts(
                    &questions_with_inputs,
                    &dropdown_answers,
                    &HashMap::new(),
                    &answer_prefix,
                );
                let msg = rental_flow::submit_purpose(
                    state.clone(),
                    guild_id,
                    user_id,
                    session_id,
                    purpose,
                    vc_id,
                    &lang,
                )
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("{e}");
                    "Error".to_string()
                });

                InteractionResponse {
                    kind: InteractionResponseType::UpdateMessage,
                    data: Some(InteractionResponseData {
                        content: Some(msg),
                        components: Some(vec![]),
                        flags: Some(MessageFlags::EPHEMERAL),
                        ..Default::default()
                    }),
                }
            } else {
                // Show modal for text questions.
                rental_flow::build_text_questions_modal(
                    &state, &lang, session_id, room_id, &text_qs,
                )
            };

            state
                .http
                .interaction(state.application_id)
                .create_response(interaction.id, &interaction.token, &response)
                .await?;
        }
        return Ok(());
    }

    // handoff_accept:{session_id}:{room_id}
    if let Some(rest) = data.custom_id.strip_prefix("handoff_accept:") {
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() == 2
            && let (Ok(session_id), Ok(_room_id)) =
                (parts[0].parse::<i32>(), parts[1].parse::<i32>())
        {
            let lang = get_lang(&state, guild_id, interaction.locale.as_deref()).await;
            let message_id = interaction
                .message
                .as_ref()
                .map(|m| m.id)
                .unwrap_or_else(|| Id::new(1));
            let channel_id = interaction
                .channel
                .as_ref()
                .map(|c| c.id)
                .unwrap_or_else(|| Id::new(1));

            state
                .http
                .interaction(state.application_id)
                .create_response(
                    interaction.id,
                    &interaction.token,
                    &InteractionResponse {
                        kind: InteractionResponseType::DeferredUpdateMessage,
                        data: None,
                    },
                )
                .await?;

            let vc_id = find_vc_for_session(&state.rental_states, session_id);
            handoff::accept_handoff(
                state,
                guild_id,
                Id::new(vc_id),
                session_id,
                user_id.get(),
                &lang,
                message_id,
                channel_id,
            )
            .await?;
        }
    }

    Ok(())
}

async fn handle_modal(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    interaction: Interaction,
) -> BotResult<()> {
    let Some(InteractionData::ModalSubmit(ref data)) = interaction.data else {
        return Ok(());
    };
    let user_id = interaction_user_id(&interaction);

    // purpose_modal:{session_id}:{room_id}
    if let Some(rest) = data.custom_id.strip_prefix("purpose_modal:") {
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() == 2
            && let (Ok(session_id), Ok(room_id)) =
                (parts[0].parse::<i32>(), parts[1].parse::<i32>())
        {
            let lang = get_lang(&state, guild_id, interaction.locale.as_deref()).await;
            let vc_id = find_vc_for_session(&state.rental_states, session_id);

            // Detect modal style: old (single purpose_text) vs new (per-question qt_{index})
            let purpose = if let Some(text) = find_in_components(&data.components, "purpose_text") {
                // Old-style: free-form text template
                text
            } else {
                // New-style: individual TextInputs for text questions after dropdown phase
                let dropdown_answers = get_dropdown_answers(&state.rental_states, session_id);
                let questions_with_inputs =
                    rental_flow::questions_with_inputs_for_room(&state, guild_id, room_id)
                        .await
                        .unwrap_or_default();

                let mut text_answers: HashMap<usize, String> = HashMap::new();
                for q in &questions_with_inputs {
                    if matches!(q.input, QuestionInput::Text) {
                        let custom_id = format!("qt_{}", q.index);
                        if let Some(val) = find_in_components(&data.components, &custom_id) {
                            if !val.is_empty() {
                                text_answers.insert(q.index, val);
                            }
                        }
                    }
                }

                let answer_prefix = state.i18n.get(&lang, &MessageKey::BotRentalAnswerPrefix);
                rental_flow::assemble_purpose_from_parts(
                    &questions_with_inputs,
                    &dropdown_answers,
                    &text_answers,
                    &answer_prefix,
                )
            };

            let msg = rental_flow::submit_purpose(
                state.clone(),
                guild_id,
                user_id,
                session_id,
                purpose,
                vc_id,
                &lang,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::error!("{e}");
                "Error".to_string()
            });

            state
                .http
                .interaction(state.application_id)
                .create_response(
                    interaction.id,
                    &interaction.token,
                    &InteractionResponse {
                        kind: InteractionResponseType::ChannelMessageWithSource,
                        data: Some(InteractionResponseData {
                            content: Some(msg),
                            flags: Some(MessageFlags::EPHEMERAL),
                            ..Default::default()
                        }),
                    },
                )
                .await?;
        }
    }
    Ok(())
}

fn is_pending_rental_host(state: &AppState, session_id: i32, user_id: u64) -> bool {
    state.rental_states.iter().any(|entry| {
        matches!(
            &entry.state,
            RentalState::AwaitingPurpose {
                session_id: pending_session_id,
                host_user_id,
                ..
            } if *pending_session_id == session_id && *host_user_id == user_id
        )
    })
}

fn find_in_components(components: &[ModalInteractionComponent], custom_id: &str) -> Option<String> {
    use twilight_model::application::interaction::modal::ModalInteractionActionRow;
    for component in components {
        match component {
            ModalInteractionComponent::TextInput(ti) if ti.custom_id == custom_id => {
                return Some(ti.value.clone());
            }
            ModalInteractionComponent::ActionRow(ModalInteractionActionRow {
                components, ..
            }) => {
                if let Some(val) = find_in_components(components, custom_id) {
                    return Some(val);
                }
            }
            _ => {}
        }
    }
    None
}

fn interaction_user_id(interaction: &Interaction) -> Id<UserMarker> {
    interaction
        .member
        .as_ref()
        .and_then(|m| m.user.as_ref())
        .map(|u| u.id)
        .or_else(|| interaction.user.as_ref().map(|u| u.id))
        .unwrap_or_else(|| Id::new(1))
}

fn check_admin_permission(interaction: &Interaction) -> bool {
    use twilight_model::guild::Permissions;
    interaction
        .member
        .as_ref()
        .and_then(|m| m.permissions)
        .map(|p| p.contains(Permissions::ADMINISTRATOR) || p.contains(Permissions::MANAGE_GUILD))
        .unwrap_or(false)
}

async fn get_lang(
    state: &AppState,
    guild_id: Id<GuildMarker>,
    discord_locale: Option<&str>,
) -> String {
    resolve_language(state, guild_id, discord_locale).await
}

fn simple_response(content: &str) -> InteractionResponse {
    InteractionResponse {
        kind: InteractionResponseType::ChannelMessageWithSource,
        data: Some(InteractionResponseData {
            content: Some(content.to_string()),
            ..Default::default()
        }),
    }
}

async fn respond_ephemeral(
    state: &AppState,
    interaction: &Interaction,
    content: &str,
) -> BotResult<()> {
    state
        .http
        .interaction(state.application_id)
        .create_response(
            interaction.id,
            &interaction.token,
            &InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(InteractionResponseData {
                    content: Some(content.to_string()),
                    flags: Some(MessageFlags::EPHEMERAL),
                    ..Default::default()
                }),
            },
        )
        .await?;
    Ok(())
}
