use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::entities::rental_sessions;
use crate::error::BotResult;
use crate::facade::{guild_settings, rental as rental_facade, room as room_facade};
use crate::i18n::MessageKey;
use crate::language::resolve_language;
use fluent_bundle::FluentArgs;
use std::collections::HashMap;
use std::sync::Arc;
use twilight_model::channel::message::AllowedMentions;
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker, MessageMarker},
};

/// Spawn a background refresh of the rental-status board message. Fire-and-forget
/// so it never adds latency to interaction responses or event handling.
pub fn trigger(state: &Arc<AppState>, guild_id: u64) {
    let state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = refresh(&state, guild_id).await {
            tracing::warn!(guild_id, "Failed to refresh rental status board: {e}");
        }
    });
}

/// Rebuild the rental-status board message in the configured rental-button channel.
/// Edits the stored message if present; otherwise (or if it was deleted) posts a new
/// one and persists its id.
pub async fn refresh(state: &Arc<AppState>, guild_id: u64) -> BotResult<()> {
    let row = with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move { guild_settings::get_rental_button_channel_row(txn, guild_id).await })
    })
    .await?;

    let Some(row) = row else {
        // Rental button channel is not configured; nothing to display.
        return Ok(());
    };
    let channel_id = Id::<ChannelMarker>::new(row.channel_id as u64);

    let lang = resolve_language(state, Id::<GuildMarker>::new(guild_id), None).await;
    let content = build_content(state, guild_id, &lang).await?;
    let allowed = AllowedMentions::default();

    if let Some(message_id) = row.message_id {
        let message_id = Id::<MessageMarker>::new(message_id as u64);
        let updated = state
            .http
            .update_message(channel_id, message_id)
            .content(Some(&content))
            .allowed_mentions(Some(&allowed))
            .await;
        if updated.is_ok() {
            return Ok(());
        }
        tracing::info!(guild_id, "Status board message is gone; recreating it");
    }

    let message = state
        .http
        .create_message(channel_id)
        .content(&content)
        .allowed_mentions(Some(&allowed))
        .await?
        .model()
        .await?;

    let new_message_id = message.id.get();
    with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move {
            guild_settings::set_rental_button_message_id(txn, guild_id, Some(new_message_id)).await
        })
    })
    .await?;
    Ok(())
}

async fn build_content(state: &Arc<AppState>, guild_id: u64, lang: &str) -> BotResult<String> {
    let rooms = with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move { room_facade::list_rooms(txn, guild_id).await })
    })
    .await?;
    let sessions = with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move { rental_facade::find_active_sessions_by_guild(txn, guild_id).await })
    })
    .await?;

    let title = state.i18n.get(lang, &MessageKey::StatusTitle);

    if rooms.is_empty() {
        let no_rooms = state.i18n.get(lang, &MessageKey::StatusNoRooms);
        return Ok(format!("**{title}**\n\n{no_rooms}"));
    }

    let by_room: HashMap<i32, &rental_sessions::Model> =
        sessions.iter().map(|s| (s.room_id, s)).collect();

    let label_available = state.i18n.get(lang, &MessageKey::StatusAvailable);
    let label_awaiting = state.i18n.get(lang, &MessageKey::StatusAwaiting);
    let label_in_use = state.i18n.get(lang, &MessageKey::StatusInUse);
    let label_handoff = state.i18n.get(lang, &MessageKey::StatusPendingHandoff);

    let mut lines = Vec::with_capacity(rooms.len());
    let mut free = 0u32;
    let mut used = 0u32;

    for room in &rooms {
        let mention = room
            .voice_channel_id
            .or(room.text_channel_id)
            .map(|id| format!("<#{id}>"))
            .unwrap_or_else(|| format!("room #{}", room.id));

        match by_room.get(&room.id) {
            None => {
                free += 1;
                lines.push(format!("🟢 {mention} {label_available}"));
            }
            Some(session) => {
                used += 1;
                let host = format!("<@{}>", session.host_user_id as u64);
                match session.state {
                    rental_sessions::STATE_AWAITING_PURPOSE => {
                        lines.push(format!("🟡 {mention} {label_awaiting} — {host}"));
                    }
                    rental_sessions::STATE_PENDING_HANDOFF => {
                        lines.push(format!("🟠 {mention} {label_handoff}"));
                    }
                    _ => {
                        lines.push(format!("🔴 {mention} {label_in_use} — {host}"));
                    }
                }
            }
        }
    }

    let mut args = FluentArgs::new();
    args.set("free", free.to_string());
    args.set("used", used.to_string());
    let summary = state
        .i18n
        .get_with_args(lang, &MessageKey::StatusSummary, Some(&args));

    Ok(format!("**{title}**\n\n{}\n\n{summary}", lines.join("\n")))
}
