use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::entities::scheduled_tasks;
use crate::facade::rental as rental_facade;
use crate::i18n::MessageKey;
use crate::language::resolve_language;
use fluent_bundle::FluentArgs;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker},
};

pub fn spawn_purpose_timeout(
    state: Arc<AppState>,
    guild_id: u64,
    voice_channel_id: u64,
    session_id: i32,
    task_id: i32,
    delay: Duration,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        tokio::time::sleep(delay).await;
        tracing::info!(session_id, "Purpose timeout fired");
        if let Err(e) =
            handle_purpose_timeout(&state, guild_id, voice_channel_id, session_id, task_id).await
        {
            tracing::error!("Purpose timeout error: {e}");
        }
        crate::rental::status::trigger(&state, guild_id);
    })
}

async fn handle_purpose_timeout(
    state: &AppState,
    guild_id: u64,
    voice_channel_id: u64,
    session_id: i32,
    task_id: i32,
) -> crate::error::BotResult<()> {
    use crate::entities::rental_sessions;

    // Skip work for sessions that are no longer pending (the user cancelled by leaving
    // the VC, or already submitted their purpose). Without this guard, a stale in-memory
    // timer that escaped abort would erroneously fire a "did not submit" report and
    // could clobber an unrelated newer rental sitting at the same VC key.
    let session = with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move {
            rental_sessions::Entity::find_by_id(session_id)
                .one(txn)
                .await
                .map_err(crate::error::BotError::from)
        })
    })
    .await?;

    let Some(session) = session else {
        if task_id != 0 {
            rental_facade::mark_task_processed(&state.db.system, task_id).await?;
        }
        return Ok(());
    };

    if session.state != rental_sessions::STATE_AWAITING_PURPOSE {
        if task_id != 0 {
            rental_facade::mark_task_processed(&state.db.system, task_id).await?;
        }
        return Ok(());
    }

    let room_id = session.room_id;

    with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move {
            rental_facade::release_session(txn, session_id).await?;
            crate::facade::room::set_room_availability(txn, room_id, true).await
        })
    })
    .await?;

    // `scheduled_tasks` is in the worker schema — UPDATE requires the system role.
    if task_id != 0 {
        rental_facade::mark_task_processed(&state.db.system, task_id).await?;
    }

    // Only remove the state entry if it still belongs to this session. A newer rental
    // started after cancellation can sit at the same (guild, vc) key.
    state
        .rental_states
        .remove_if(&(guild_id, voice_channel_id), |_, entry| {
            entry.session_id() == session_id
        });

    let lang = resolve_language(state, Id::<GuildMarker>::new(guild_id), None).await;

    let report_channel_id = with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(
            async move { crate::facade::guild_settings::get_report_channel(txn, guild_id).await },
        )
    })
    .await?;

    if let Some(ch_id) = report_channel_id {
        let mut args = FluentArgs::new();
        args.set("user", format!("<#{voice_channel_id}>"));
        let msg = state
            .i18n
            .get_with_args(&lang, &MessageKey::BotRentalReport, Some(&args));
        state
            .http
            .create_message(Id::<ChannelMarker>::new(ch_id as u64))
            .content(&msg)
            .await?;
    }
    Ok(())
}

pub fn spawn_handoff_timeout(
    state: Arc<AppState>,
    guild_id: u64,
    voice_channel_id: u64,
    session_id: i32,
    room_id: i32,
    delay: Duration,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        tokio::time::sleep(delay).await;
        tracing::info!(session_id, "Handoff timeout fired - releasing room");
        if let Err(e) =
            handle_handoff_timeout(&state, guild_id, voice_channel_id, session_id, room_id).await
        {
            tracing::error!("Handoff timeout error: {e}");
        }
        crate::rental::status::trigger(&state, guild_id);
    })
}

async fn handle_handoff_timeout(
    state: &AppState,
    guild_id: u64,
    voice_channel_id: u64,
    session_id: i32,
    room_id: i32,
) -> crate::error::BotResult<()> {
    with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move {
            rental_facade::release_session(txn, session_id).await?;
            crate::facade::room::set_room_availability(txn, room_id, true).await
        })
    })
    .await?;
    state.rental_states.remove(&(guild_id, voice_channel_id));
    Ok(())
}

pub async fn restore_pending_timeouts(state: Arc<AppState>) {
    use crate::entities::scheduled_tasks::TASK_TYPE_TIMEOUT_NOTIFICATION;

    let tasks = scheduled_tasks::Entity::find()
        .filter(scheduled_tasks::Column::Processed.eq(false))
        .filter(scheduled_tasks::Column::TaskType.eq(TASK_TYPE_TIMEOUT_NOTIFICATION))
        .all(&state.db.system)
        .await;

    let tasks = match tasks {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to load pending tasks: {e}");
            return;
        }
    };

    let now = chrono::Utc::now();
    for task in tasks {
        let delay = {
            let diff = task.schedule_datetime.signed_duration_since(now);
            if diff.num_milliseconds() > 0 {
                Duration::from_millis(diff.num_milliseconds() as u64)
            } else {
                Duration::from_secs(0)
            }
        };

        tracing::info!(
            task_id = task.id,
            "Restoring timeout task, fires in {}s",
            delay.as_secs()
        );

        if let Some(session_id) = task.rental_session_id {
            let state_clone = state.clone();
            let task_id = task.id;
            let guild_id = task.guild_id as u64;
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                if let Err(e) = restore_fire(&state_clone, guild_id, session_id, task_id).await {
                    tracing::error!("Restored timeout error: {e}");
                }
                crate::rental::status::trigger(&state_clone, guild_id);
            });
        }
    }
}

async fn restore_fire(
    state: &AppState,
    guild_id: u64,
    session_id: i32,
    task_id: i32,
) -> crate::error::BotResult<()> {
    use crate::entities::rental_sessions;

    let session = with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move {
            rental_sessions::Entity::find_by_id(session_id)
                .one(txn)
                .await
                .map_err(crate::error::BotError::from)
        })
    })
    .await?;

    // The session may have advanced past AwaitingPurpose since this task was
    // scheduled (e.g. the user submitted their purpose). Only release sessions
    // still awaiting a purpose; otherwise just retire the stale scheduled task.
    let still_awaiting = session
        .as_ref()
        .map(|s| s.state == rental_sessions::STATE_AWAITING_PURPOSE)
        .unwrap_or(false);
    let room_id = session.as_ref().map(|s| s.room_id);

    if still_awaiting {
        with_guild_context(&state.db.guild, guild_id, |txn| {
            Box::pin(async move {
                rental_facade::release_session(txn, session_id).await?;
                if let Some(room_id) = room_id {
                    crate::facade::room::set_room_availability(txn, room_id, true).await?;
                }
                Ok(())
            })
        })
        .await?;
    }

    // `scheduled_tasks` is in the worker schema — UPDATE requires the system role.
    rental_facade::mark_task_processed(&state.db.system, task_id).await
}
