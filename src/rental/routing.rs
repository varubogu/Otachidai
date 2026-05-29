//! 利用目的の自動投稿ロジック。
//!
//! 質問モーダル送信後に呼ばれ、プリセットの `routing_key_index` が指す質問の回答で
//! `rental_routing_rules` を引き、一致したチャンネル（無ければフォールバック）へ投稿する。
//! 投稿失敗はログに留め、レンタル成立処理は止めない。

use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::entities::rental_question_presets;
use crate::error::BotResult;
use crate::facade::{
    guild_settings as gs_facade, question_preset as qp_facade, rental as rental_facade,
    room as room_facade, routing as routing_facade,
};
use crate::i18n::MessageKey;
use crate::rental::template;
use fluent_bundle::FluentArgs;
use sea_orm::EntityTrait;
use std::collections::HashMap;
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker, UserMarker},
};

/// Everything needed to render and route a single purpose-post message.
pub struct PostRequest {
    pub guild_id: Id<GuildMarker>,
    pub user_id: Id<UserMarker>,
    pub session_id: i32,
    /// Pre-assembled purpose body (the existing `{{answers}}` payload).
    pub assembled_purpose: String,
    /// Question text → answer (covers both dropdowns and text inputs).
    pub answers_by_name: HashMap<String, String>,
    /// 0-based answers indexed by question position (so question_1 == [0]).
    pub answers_by_index: Vec<String>,
    pub lang: String,
}

/// Post the purpose for a freshly-activated rental session.
///
/// Errors are logged and swallowed — the calling code already committed the rental, so we
/// don't want a missing channel or template hiccup to surface as a user-visible failure.
pub async fn post_purpose(state: &AppState, req: PostRequest) {
    if let Err(e) = post_purpose_inner(state, req).await {
        tracing::warn!("auto-post for rental failed: {e}");
    }
}

async fn post_purpose_inner(state: &AppState, req: PostRequest) -> BotResult<()> {
    let PostRequest {
        guild_id,
        user_id,
        session_id,
        assembled_purpose,
        answers_by_name,
        answers_by_index,
        lang,
    } = req;

    // 1) Load session + room + preset (all under RLS).
    let resolved = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            let session = crate::entities::rental_sessions::Entity::find_by_id(session_id)
                .one(txn)
                .await?;
            let Some(session) = session else {
                return Ok::<_, crate::error::BotError>(None);
            };
            let room = room_facade::find_room_by_id(txn, session.room_id).await?;
            let preset_id = room.as_ref().and_then(|r| r.question_preset_id);
            let preset: Option<rental_question_presets::Model> = match preset_id {
                Some(pid) => qp_facade::find_by_id(txn, pid).await?,
                None => None,
            };
            Ok(Some((session, room, preset)))
        })
    })
    .await?;

    let Some((_session, room, preset)) = resolved else {
        return Ok(());
    };

    // 2) Compute the routing-key answer (if any).
    let route_value: Option<String> = preset
        .as_ref()
        .and_then(|p| p.routing_key_index)
        .and_then(|idx| {
            let i = idx as usize;
            answers_by_index.get(i).cloned()
        })
        .filter(|s| !s.is_empty());

    // 3) Resolve target channel + template.
    let target = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        let preset_id = preset.as_ref().map(|p| p.id);
        let route_value = route_value.clone();
        let guild_raw = guild_id.get();
        Box::pin(async move {
            // Per-preset matching only kicks in when we have a preset AND a non-empty
            // routing-key answer.
            if let (Some(pid), Some(value)) = (preset_id, route_value.as_ref())
                && let Some(rule) = routing_facade::find_rule(txn, guild_raw, pid, value).await?
            {
                return Ok::<_, crate::error::BotError>(Some((
                    rule.channel_id,
                    rule.template,
                    value.clone(),
                )));
            }
            // Fallback channel.
            let fb = gs_facade::get_rental_post_fallback(txn, guild_raw).await?;
            Ok(fb.map(|(ch, tpl)| (ch, tpl, String::new())))
        })
    })
    .await?;

    let Some((channel_id, template_str, matched_when)) = target else {
        tracing::debug!(
            session_id,
            "no routing rule and no fallback channel — skipping auto-post",
        );
        return Ok(());
    };

    // 4) Render template (falling back to the localised default).
    let room_mention = room
        .as_ref()
        .and_then(|r| r.text_channel_id.or(r.voice_channel_id))
        .map(|id| format!("<#{id}>"))
        .unwrap_or_else(|| "room".to_string());
    let preset_name = preset.as_ref().map(|p| p.name.clone()).unwrap_or_default();
    let user_mention = format!("<@{}>", user_id.get());

    let rendered = match template_str.as_deref() {
        Some(tpl) => render_template(
            tpl,
            &user_mention,
            &room_mention,
            &matched_when,
            &preset_name,
            &answers_by_index,
            &answers_by_name,
            &assembled_purpose,
        ),
        None => default_message(
            state,
            &lang,
            &user_mention,
            &room_mention,
            &assembled_purpose,
        ),
    };

    // 5) Post.
    state
        .http
        .create_message(Id::<ChannelMarker>::new(channel_id as u64))
        .content(&rendered)
        .await?;

    Ok(())
}

fn render_template(
    tpl: &str,
    user: &str,
    room: &str,
    when: &str,
    preset: &str,
    answers_by_index: &[String],
    answers_by_name: &HashMap<String, String>,
    answers_text: &str,
) -> String {
    // Templates are pre-validated at upload time; a parse failure here means the DB row
    // was somehow saved with an invalid template — surface the literal so the operator
    // can spot it instead of silently dropping the message.
    let parsed = match template::parse(tpl) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("stored template failed to parse: {e}");
            return tpl.to_string();
        }
    };
    let ctx = template::RenderContext {
        user: user.to_string(),
        room: room.to_string(),
        when: when.to_string(),
        preset: preset.to_string(),
        questions: answers_by_index.to_vec(),
        answers_by_name: answers_by_name.clone(),
        answers_text: answers_text.to_string(),
    };
    template::render(&parsed, &ctx)
}

fn default_message(state: &AppState, lang: &str, user: &str, room: &str, answers: &str) -> String {
    let mut args = FluentArgs::new();
    args.set("user", user.to_string());
    args.set("room", room.to_string());
    args.set("answers", answers.to_string());
    state
        .i18n
        .get_with_args(lang, &MessageKey::BotRentalPostDefaultTemplate, Some(&args))
}

/// Best-effort force-release used by the YAML applier when an active session sits on a
/// room that's about to be deleted.
///
/// The DB rows go away via the room cascade; this function is responsible for the parts
/// the DB can't do for us: aborting Tokio timeout tasks, dropping in-memory state map
/// entries, and DMing the affected host so they know why they lost their VC.
///
/// Best-effort: any individual notification failure is logged but doesn't poison the
/// surrounding YAML apply.
pub async fn force_release_for_rooms(
    state: &AppState,
    guild_id: u64,
    affected_room_ids: &[i32],
    lang: &str,
) {
    if affected_room_ids.is_empty() {
        return;
    }

    // Collect (vc_channel_id, host_user_id) pairs from the in-memory map by room_id.
    let mut victims: Vec<(u64, u64)> = Vec::new();
    {
        let mut keys_to_remove: Vec<(u64, u64)> = Vec::new();
        for entry in state.rental_states.iter() {
            if affected_room_ids.contains(&entry.room_id) && entry.key().0 == guild_id {
                entry.abort_timeout();
                let host = match &entry.state {
                    crate::rental::state_machine::RentalState::AwaitingPurpose {
                        host_user_id,
                        ..
                    } => *host_user_id,
                    crate::rental::state_machine::RentalState::Active { host_user_id, .. } => {
                        *host_user_id
                    }
                    crate::rental::state_machine::RentalState::PendingHandoff { .. } => 0,
                };
                victims.push((entry.key().1, host));
                keys_to_remove.push(*entry.key());
            }
        }
        for k in keys_to_remove {
            state.rental_states.remove(&k);
        }
    }

    // Notify hosts. Sessions are about to be cascade-deleted with the rooms, so we don't
    // bother updating their DB state — but we DO need to mark any scheduled_tasks rows
    // processed so a future restart doesn't try to fire timeouts for the gone sessions.
    // Worker schema requires the system role.
    if let Ok(sessions) =
        rental_facade::find_active_sessions_by_guild(&state.db.system, guild_id).await
    {
        for s in sessions {
            if affected_room_ids.contains(&s.room_id)
                && let Err(e) =
                    rental_facade::mark_session_tasks_processed(&state.db.system, s.id).await
            {
                tracing::warn!("force_release: mark_session_tasks_processed failed: {e}");
            }
        }
    }

    let msg = state.i18n.get(lang, &MessageKey::BotRentalForceReleased);
    for (vc, host) in victims {
        if host == 0 {
            continue;
        }
        let channel = Id::<ChannelMarker>::new(vc);
        if let Err(e) = state
            .http
            .create_message(channel)
            .content(&format!("<@{host}> {msg}"))
            .await
        {
            tracing::warn!("force_release: notify in vc {vc} failed: {e}");
        }
    }
}
