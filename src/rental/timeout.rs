use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::entities::scheduled_tasks;
use crate::facade::rental as rental_facade;
use crate::i18n::MessageKey;
use fluent_bundle::FluentArgs;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use twilight_model::id::{Id, marker::ChannelMarker};

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
    })
}

async fn handle_purpose_timeout(
    state: &AppState,
    guild_id: u64,
    voice_channel_id: u64,
    session_id: i32,
    task_id: i32,
) -> crate::error::BotResult<()> {
    let room_id = with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move { get_room_id_for_session(txn, session_id).await })
    })
    .await?;

    with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move {
            rental_facade::release_session(txn, session_id).await?;
            if task_id != 0 {
                rental_facade::mark_task_processed(txn, task_id).await?;
            }
            crate::facade::room::set_room_availability(txn, room_id, true).await
        })
    })
    .await?;

    state.rental_states.remove(&(guild_id, voice_channel_id));

    let lang = with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move { crate::facade::guild_settings::get_language(txn, guild_id).await })
    })
    .await
    .unwrap_or_else(|_| "en".to_string());

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

async fn get_room_id_for_session<C: sea_orm::ConnectionTrait>(
    db: &C,
    session_id: i32,
) -> crate::error::BotResult<i32> {
    use crate::entities::rental_sessions;
    rental_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await
        .map_err(crate::error::BotError::from)?
        .map(|s| s.room_id)
        .ok_or_else(|| crate::error::BotError::NotFound(format!("session {session_id}")))
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
    let room_id = with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move { get_room_id_for_session(txn, session_id).await })
    })
    .await?;

    with_guild_context(&state.db.guild, guild_id, |txn| {
        Box::pin(async move {
            rental_facade::release_session(txn, session_id).await?;
            rental_facade::mark_task_processed(txn, task_id).await?;
            crate::facade::room::set_room_availability(txn, room_id, true).await
        })
    })
    .await
}
