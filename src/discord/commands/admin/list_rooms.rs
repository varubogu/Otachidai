use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::BotResult;
use crate::facade::{group as group_facade, question_preset, room as room_facade};
use crate::i18n::MessageKey;
use fluent_bundle::FluentArgs;
use std::collections::HashMap;
use std::sync::Arc;
use twilight_model::id::{Id, marker::GuildMarker};

/// Discord rejects messages longer than 2000 characters.
const MAX_MESSAGE_LEN: usize = 2000;

pub async fn handle(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    lang: &str,
) -> BotResult<String> {
    let (rooms, presets, groups) = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            let rooms = room_facade::list_rooms(txn, guild_id.get()).await?;
            let presets = question_preset::list_by_guild(txn, guild_id.get()).await?;
            let groups = group_facade::list_groups(txn, guild_id.get()).await?;
            Ok((rooms, presets, groups))
        })
    })
    .await?;

    if rooms.is_empty() {
        return Ok(state.i18n.get(lang, &MessageKey::AdminRoomListEmpty));
    }

    let preset_names: HashMap<i32, String> = presets.into_iter().map(|p| (p.id, p.name)).collect();
    let group_names: HashMap<i32, String> = groups.into_iter().map(|g| (g.id, g.name)).collect();
    let none_label = state.i18n.get(lang, &MessageKey::AdminRoomListNone);

    let mut out = state.i18n.get(lang, &MessageKey::AdminRoomListHeader);

    for room in &rooms {
        let mut channels = Vec::new();
        if let Some(tid) = room.text_channel_id {
            channels.push(format!("<#{}>", tid));
        }
        if let Some(vid) = room.voice_channel_id {
            channels.push(format!("<#{}>", vid));
        }
        let channels = if channels.is_empty() {
            none_label.clone()
        } else {
            channels.join(" / ")
        };

        let preset = room
            .question_preset_id
            .and_then(|id| preset_names.get(&id).cloned())
            .unwrap_or_else(|| none_label.clone());
        let group = room
            .group_id
            .and_then(|id| group_names.get(&id).cloned())
            .unwrap_or_else(|| none_label.clone());

        let mut args = FluentArgs::new();
        args.set("id", room.id.to_string());
        args.set("channels", channels);
        args.set("preset", preset);
        args.set("group", group);

        out.push('\n');
        out.push_str(
            &state
                .i18n
                .get_with_args(lang, &MessageKey::AdminRoomListItem, Some(&args)),
        );
    }

    Ok(truncate(out))
}

fn truncate(mut s: String) -> String {
    if s.chars().count() <= MAX_MESSAGE_LEN {
        return s;
    }
    let cut: String = s.chars().take(MAX_MESSAGE_LEN - 1).collect();
    s = cut;
    s.push('…');
    s
}
