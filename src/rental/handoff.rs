use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::BotResult;
use crate::i18n::MessageKey;
use crate::rental::state_machine::{RentalState, RentalStateEntry};
use crate::rental::timeout::spawn_handoff_timeout;
use fluent_bundle::FluentArgs;
use std::sync::Arc;
use std::time::Duration;
use twilight_model::channel::message::component::{ActionRow, Button, ButtonStyle, Component};
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker, MessageMarker},
};

pub const HANDOFF_TIMEOUT_SECS: u64 = 300;

pub async fn initiate_handoff(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    voice_channel_id: Id<ChannelMarker>,
    session_id: i32,
    room_id: i32,
    lang: &str,
) -> BotResult<()> {
    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move { crate::facade::rental::set_pending_handoff(txn, session_id).await })
    })
    .await?;

    let msg = state.i18n.get(lang, &MessageKey::BotHandoffPrompt);
    let button_label = state.i18n.get(lang, &MessageKey::BotHandoffTakeOver);

    let button = Component::Button(Button {
        id: None,
        custom_id: Some(format!("handoff_accept:{session_id}:{room_id}")),
        disabled: false,
        emoji: None,
        label: Some(button_label),
        style: ButtonStyle::Primary,
        url: None,
        sku_id: None,
    });
    let row = Component::ActionRow(ActionRow {
        id: None,
        components: vec![button],
    });

    state
        .http
        .create_message(voice_channel_id)
        .content(&msg)
        .components(&[row])
        .await?;

    let timeout = spawn_handoff_timeout(
        state.clone(),
        guild_id.get(),
        voice_channel_id.get(),
        session_id,
        room_id,
        Duration::from_secs(HANDOFF_TIMEOUT_SECS),
    );

    let key = (guild_id.get(), voice_channel_id.get());
    state.rental_states.insert(
        key,
        RentalStateEntry {
            state: RentalState::PendingHandoff {
                session_id,
                timeout_task: timeout,
            },
            room_id,
        },
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn accept_handoff(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    voice_channel_id: Id<ChannelMarker>,
    session_id: i32,
    new_host_id: u64,
    lang: &str,
    original_message_id: Id<MessageMarker>,
    channel_id: Id<ChannelMarker>,
) -> BotResult<()> {
    let key = (guild_id.get(), voice_channel_id.get());

    if let Some(entry) = state.rental_states.get(&key) {
        entry.abort_timeout();
    }

    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            crate::facade::rental::transfer_host(txn, session_id, new_host_id)
                .await
                .map(|_| ())
        })
    })
    .await?;

    let mut args = FluentArgs::new();
    args.set("user", format!("<@{new_host_id}>"));
    let msg = state
        .i18n
        .get_with_args(lang, &MessageKey::BotHandoffAccepted, Some(&args));

    state
        .http
        .update_message(channel_id, original_message_id)
        .content(Some(&msg))
        .components(Some(&[]))
        .await?;

    let room_id = state
        .rental_states
        .get(&key)
        .map(|e| e.room_id)
        .unwrap_or(0);

    state.rental_states.insert(
        key,
        RentalStateEntry {
            state: RentalState::Active {
                session_id,
                host_user_id: new_host_id,
            },
            room_id,
        },
    );

    Ok(())
}
