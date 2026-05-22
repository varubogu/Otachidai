use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::entities::{rental_sessions, rooms};
use crate::error::BotResult;
use crate::facade::{
    group as group_facade, guild_settings, rental as rental_facade, room as room_facade,
};
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

/// Spawn a background refresh of the rental-status boards. Fire-and-forget
/// so it never adds latency to interaction responses or event handling.
pub fn trigger(state: &Arc<AppState>, guild_id: u64) {
    let state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = refresh(&state, guild_id).await {
            tracing::warn!(guild_id, "Failed to refresh rental status boards: {e}");
        }
    });
}

/// Rebuild every rental-status board for a guild: the guild-wide board (ungrouped
/// rooms, posted in the rental-button channel) and one board per room group.
pub async fn refresh(state: &Arc<AppState>, guild_id: u64) -> BotResult<()> {
    let lang = resolve_language(state, Id::<GuildMarker>::new(guild_id), None).await;

    let sessions = with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move { rental_facade::find_active_sessions_by_guild(txn, guild_id).await })
    })
    .await?;
    let groups = with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move { group_facade::list_groups(txn, guild_id).await })
    })
    .await?;

    refresh_guild_board(state, guild_id, &lang, &sessions).await?;

    for group in groups {
        if let Err(e) = refresh_group_board(state, guild_id, &lang, &group, &sessions).await {
            tracing::warn!(
                guild_id,
                group_id = group.id,
                "Failed to refresh group status board: {e}"
            );
        }
    }
    Ok(())
}

/// Refresh the guild-wide board (ungrouped rooms) in the rental-button channel.
async fn refresh_guild_board(
    state: &Arc<AppState>,
    guild_id: u64,
    lang: &str,
    sessions: &[rental_sessions::Model],
) -> BotResult<()> {
    let row = with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move { guild_settings::get_rental_button_channel_row(txn, guild_id).await })
    })
    .await?;

    let Some(row) = row else {
        // Rental button channel is not configured; nothing to display.
        return Ok(());
    };
    let channel_id = Id::<ChannelMarker>::new(row.channel_id as u64);

    let rooms = with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move { room_facade::list_ungrouped_rooms(txn, guild_id).await })
    })
    .await?;

    let title = state.i18n.get(lang, &MessageKey::StatusTitle);
    let content = build_content(state, lang, &title, &rooms, sessions);

    let stored = row.message_id.map(|id| id as u64);
    let new_id = render_board(state, channel_id, stored, &content).await?;

    if Some(new_id) != stored {
        with_guild_context(&state.db.guild, guild_id, |txn| {
            Box::pin(async move {
                guild_settings::set_rental_button_message_id(txn, guild_id, Some(new_id)).await
            })
        })
        .await?;
    }
    Ok(())
}

/// Refresh a single room group's board in that group's configured channel.
async fn refresh_group_board(
    state: &Arc<AppState>,
    guild_id: u64,
    lang: &str,
    group: &crate::entities::room_groups::Model,
    sessions: &[rental_sessions::Model],
) -> BotResult<()> {
    let channel_id = Id::<ChannelMarker>::new(group.channel_id as u64);
    let group_id = group.id;

    let rooms = with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move { room_facade::list_rooms_by_group(txn, guild_id, group_id).await })
    })
    .await?;

    let content = build_content(state, lang, &group.name, &rooms, sessions);

    let stored = group.message_id.map(|id| id as u64);
    let new_id = render_board(state, channel_id, stored, &content).await?;

    if Some(new_id) != stored {
        with_guild_context(&state.db.guild, guild_id, |txn| {
            Box::pin(async move {
                group_facade::set_group_message_id(txn, group_id, Some(new_id)).await
            })
        })
        .await?;
    }
    Ok(())
}

/// Edit the stored board message if present; otherwise (or if it was deleted)
/// post a new one. Returns the id of the message that now holds the board.
async fn render_board(
    state: &Arc<AppState>,
    channel_id: Id<ChannelMarker>,
    current: Option<u64>,
    content: &str,
) -> BotResult<u64> {
    let allowed = AllowedMentions::default();

    if let Some(message_id) = current {
        let updated = state
            .http
            .update_message(channel_id, Id::<MessageMarker>::new(message_id))
            .content(Some(content))
            .allowed_mentions(Some(&allowed))
            .await;
        if updated.is_ok() {
            return Ok(message_id);
        }
        tracing::info!("Status board message is gone; recreating it");
    }

    let message = state
        .http
        .create_message(channel_id)
        .content(content)
        .allowed_mentions(Some(&allowed))
        .await?
        .model()
        .await?;
    Ok(message.id.get())
}

fn build_content(
    state: &Arc<AppState>,
    lang: &str,
    title: &str,
    rooms: &[rooms::Model],
    sessions: &[rental_sessions::Model],
) -> String {
    if rooms.is_empty() {
        let no_rooms = state.i18n.get(lang, &MessageKey::StatusNoRooms);
        return format!("**{title}**\n\n{no_rooms}");
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

    for room in rooms {
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

    format!("**{title}**\n\n{}\n\n{summary}", lines.join("\n"))
}
