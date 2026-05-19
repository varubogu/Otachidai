use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::entities::{rental_sessions, rooms};
use crate::error::{BotError, BotResult};
use crate::facade::{rental as rental_facade, room as room_facade};
use crate::language::resolve_language;
use crate::rental::handoff;
use crate::rental::state_machine::{RentalState, RentalStateEntry};
use sea_orm::EntityTrait;
use std::sync::Arc;
use twilight_model::id::{Id, marker::GuildMarker};

/// On startup (or gateway reconnect), reconcile persisted rental sessions against
/// the actual voice-channel occupancy. The in-memory `rental_states` map is never
/// rebuilt from the DB, so without this an `Active`/`PendingHandoff` rental from
/// before a restart would never be released when its host leaves.
pub async fn reconcile_guild(state: &Arc<AppState>, guild_id: Id<GuildMarker>) -> BotResult<()> {
    let gid = guild_id.get();
    let sessions = with_guild_context(&state.db.guild, gid, |txn| {
        Box::pin(async move { rental_facade::find_active_sessions_by_guild(txn, gid).await })
    })
    .await?;

    if !sessions.is_empty() {
        let lang = resolve_language(state, guild_id, None).await;
        for session in sessions {
            if let Err(e) = reconcile_session(state, guild_id, &session, &lang).await {
                tracing::error!(session_id = session.id, "Failed to reconcile rental: {e}");
            }
        }
    }

    // Refresh the status board so it reflects current state after a restart.
    crate::rental::status::trigger(state, guild_id.get());

    Ok(())
}

async fn reconcile_session(
    state: &Arc<AppState>,
    guild_id: Id<GuildMarker>,
    session: &rental_sessions::Model,
    lang: &str,
) -> BotResult<()> {
    let gid = guild_id.get();
    let room_id = session.room_id;

    let room = with_guild_context(&state.db.guild, gid, |txn| {
        Box::pin(async move {
            rooms::Entity::find_by_id(room_id)
                .one(txn)
                .await
                .map_err(BotError::from)
        })
    })
    .await?;

    let vc = room
        .as_ref()
        .and_then(|r| r.voice_channel_id)
        .map(|id| id as u64)
        .unwrap_or(0);

    let key = (gid, vc);
    // On a gateway reconnect the in-memory state is still intact; don't clobber it.
    if state.rental_states.contains_key(&key) {
        return Ok(());
    }

    let host_present = state
        .voice_occupancy
        .channel_for_user(gid, session.host_user_id as u64)
        == Some(vc);
    // `vc == 0` means the room has no voice channel registered, so nobody can be present.
    let anyone_present = vc != 0 && state.voice_occupancy.has_users(gid, vc);

    match session.state {
        rental_sessions::STATE_AWAITING_PURPOSE => {
            // Purpose was never submitted; dropdown answers live only in memory and
            // are lost on restart, so cancel the request and let the user re-apply.
            release(state, guild_id, session.id, room_id).await?;
            tracing::info!(
                session_id = session.id,
                "Reconciled awaiting-purpose rental -> released"
            );
        }
        rental_sessions::STATE_ACTIVE => {
            if host_present {
                state.rental_states.insert(
                    key,
                    RentalStateEntry {
                        state: RentalState::Active {
                            session_id: session.id,
                            host_user_id: session.host_user_id as u64,
                        },
                        room_id,
                    },
                );
                tracing::info!(
                    session_id = session.id,
                    vc,
                    "Reconciled active rental (host present)"
                );
            } else if anyone_present {
                handoff::initiate_handoff(
                    state.clone(),
                    guild_id,
                    Id::new(vc),
                    session.id,
                    room_id,
                    lang,
                )
                .await?;
                tracing::info!(
                    session_id = session.id,
                    vc,
                    "Reconciled active rental -> handoff (host absent)"
                );
            } else {
                release(state, guild_id, session.id, room_id).await?;
                tracing::info!(
                    session_id = session.id,
                    vc,
                    "Reconciled active rental -> released (empty)"
                );
            }
        }
        rental_sessions::STATE_PENDING_HANDOFF => {
            if anyone_present {
                handoff::initiate_handoff(
                    state.clone(),
                    guild_id,
                    Id::new(vc),
                    session.id,
                    room_id,
                    lang,
                )
                .await?;
                tracing::info!(
                    session_id = session.id,
                    vc,
                    "Reconciled pending-handoff rental -> handoff"
                );
            } else {
                release(state, guild_id, session.id, room_id).await?;
                tracing::info!(
                    session_id = session.id,
                    vc,
                    "Reconciled pending-handoff rental -> released (empty)"
                );
            }
        }
        _ => {}
    }
    Ok(())
}

/// Release a session directly (without an in-memory `rental_states` entry, which
/// `rental::flow::release_rental` requires).
async fn release(
    state: &Arc<AppState>,
    guild_id: Id<GuildMarker>,
    session_id: i32,
    room_id: i32,
) -> BotResult<()> {
    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            rental_facade::release_session(txn, session_id).await?;
            rental_facade::mark_session_tasks_processed(txn, session_id).await?;
            room_facade::set_room_availability(txn, room_id, true).await
        })
    })
    .await
}
