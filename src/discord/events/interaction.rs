use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::BotResult;
use crate::facade::question_preset::{self, QuestionInput};
use crate::i18n::MessageKey;
use crate::language::resolve_language;
use crate::rental::state_machine::find_vc_for_session;
use crate::rental::{flow as rental_flow, handoff, state_machine::RentalState};
use std::collections::HashMap;
use std::sync::Arc;
use twilight_model::{
    application::command::{CommandOptionChoice, CommandOptionChoiceValue},
    application::interaction::{
        Interaction, InteractionData, InteractionType, application_command::CommandOptionValue,
        modal::ModalInteractionComponent,
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
        InteractionType::ApplicationCommandAutocomplete => {
            handle_autocomplete(state, guild_id, interaction).await
        }
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
            | "list_question_presets"
            | "delete_question_preset"
            | "register_room"
            | "list_rooms"
            | "delete_room"
            | "register_group"
            | "delete_group"
            | "set_room_group"
            | "set_room_preset"
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
        "list_question_presets" => {
            let msg = crate::discord::commands::admin::list_question_presets::handle(
                state.clone(),
                guild_id,
                &lang,
            )
            .await
            .unwrap_or_else(|e| {
                tracing::error!("{e}");
                "Error".to_string()
            });
            simple_response(&msg)
        }
        "delete_question_preset" => {
            let msg = crate::discord::commands::admin::delete_question_preset::handle(
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
        "list_rooms" => {
            let msg =
                crate::discord::commands::admin::list_rooms::handle(state.clone(), guild_id, &lang)
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
        "set_room_preset" => {
            let msg = crate::discord::commands::admin::set_room_preset::handle(
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

async fn handle_autocomplete(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    interaction: Interaction,
) -> BotResult<()> {
    let Some(InteractionData::ApplicationCommand(ref data)) = interaction.data else {
        return Ok(());
    };

    // Locate the option the user is currently typing into.
    let focused = data.options.iter().find_map(|o| match &o.value {
        CommandOptionValue::Focused(value, _) => Some((o.name.as_str(), value.as_str())),
        _ => None,
    });

    let choices = match (data.name.as_str(), focused) {
        ("register_room", Some(("question_preset", current)))
        | ("set_room_preset", Some(("question_preset", current)))
        | ("delete_question_preset", Some(("name", current))) => {
            question_preset_choices(&state, guild_id, current).await
        }
        _ => Vec::new(),
    };

    let response = InteractionResponse {
        kind: InteractionResponseType::ApplicationCommandAutocompleteResult,
        data: Some(InteractionResponseData {
            choices: Some(choices),
            ..Default::default()
        }),
    };

    state
        .http
        .interaction(state.application_id)
        .create_response(interaction.id, &interaction.token, &response)
        .await?;
    Ok(())
}

/// Build `"id:name"` suggestions for the guild's question presets, filtered by what the
/// user has typed so far. Discord caps autocomplete at 25 choices and 100-char names.
async fn question_preset_choices(
    state: &AppState,
    guild_id: Id<GuildMarker>,
    current: &str,
) -> Vec<CommandOptionChoice> {
    let presets = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move { question_preset::list_by_guild(txn, guild_id.get()).await })
    })
    .await
    .unwrap_or_else(|e| {
        tracing::error!("{e}");
        Vec::new()
    });

    let needle = current.trim().to_lowercase();
    presets
        .into_iter()
        .filter(|preset| {
            needle.is_empty()
                || question_preset::format_ref_label(preset)
                    .to_lowercase()
                    .contains(&needle)
        })
        .take(25)
        .map(|preset| {
            // Discord caps the choice name/value at 100 characters; truncate on a char
            // boundary so multi-byte (e.g. Japanese) names don't panic.
            let label: String = question_preset::format_ref_label(&preset)
                .chars()
                .take(100)
                .collect();
            CommandOptionChoice {
                name: label.clone(),
                name_localizations: None,
                value: CommandOptionChoiceValue::String(label),
            }
        })
        .collect()
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

    // answer:{session_id}:{room_id} — open the unified question modal
    if let Some(rest) = data.custom_id.strip_prefix("answer:") {
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() == 2
            && let (Ok(session_id), Ok(room_id)) =
                (parts[0].parse::<i32>(), parts[1].parse::<i32>())
        {
            let lang = get_lang(&state, guild_id, interaction.locale.as_deref()).await;
            let response = if is_pending_rental_host(&state, session_id, user_id.get()) {
                rental_flow::build_unified_modal_for_room(
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

            // The unified modal submits every answer at once. Each input is keyed `mq_{index}`:
            // select menus carry their chosen value, text inputs their typed value.
            let mut submitted: HashMap<String, String> = HashMap::new();
            for component in &data.components {
                collect_modal_value(component, &mut submitted);
            }

            let questions_with_inputs =
                rental_flow::questions_with_inputs_for_room(&state, guild_id, room_id)
                    .await
                    .unwrap_or_default();

            let mut dropdown_answers: Vec<Option<String>> = vec![None; 10];
            let mut text_answers: HashMap<usize, String> = HashMap::new();
            for q in &questions_with_inputs {
                let Some(value) = submitted.get(&format!("mq_{}", q.index)) else {
                    continue;
                };
                if value.is_empty() {
                    continue;
                }
                match q.input {
                    QuestionInput::Dropdown(_) if q.index < dropdown_answers.len() => {
                        dropdown_answers[q.index] = Some(value.clone());
                    }
                    QuestionInput::Dropdown(_) => {}
                    QuestionInput::Text => {
                        text_answers.insert(q.index, value.clone());
                    }
                }
            }

            let answer_prefix = state.i18n.get(&lang, &MessageKey::BotRentalAnswerPrefix);
            let purpose = rental_flow::assemble_purpose_from_parts(
                &questions_with_inputs,
                &dropdown_answers,
                &text_answers,
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

/// True when `session_id` has an in-memory `AwaitingPurpose` entry hosted by `user_id`.
/// Guards the answer modal so only the rental host can fill it in.
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

/// Recursively collect each input's submitted value from a modal component tree, keyed by
/// custom id. Text inputs contribute their `value`; string selects their first chosen value.
/// `Label` and `ActionRow` are layout wrappers we descend into.
fn collect_modal_value(component: &ModalInteractionComponent, out: &mut HashMap<String, String>) {
    match component {
        ModalInteractionComponent::TextInput(ti) => {
            out.insert(ti.custom_id.clone(), ti.value.clone());
        }
        ModalInteractionComponent::StringSelect(select) => {
            if let Some(value) = select.values.first() {
                out.insert(select.custom_id.clone(), value.clone());
            }
        }
        ModalInteractionComponent::Label(label) => collect_modal_value(&label.component, out),
        ModalInteractionComponent::ActionRow(action_row) => {
            for inner in &action_row.components {
                collect_modal_value(inner, out);
            }
        }
        _ => {}
    }
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
