use crate::entities::rental_sessions::{
    self, STATE_ACTIVE, STATE_AWAITING_PURPOSE, STATE_PENDING_HANDOFF, STATE_RELEASED,
};
use crate::entities::scheduled_tasks::{self, TASK_TYPE_TIMEOUT_NOTIFICATION};
use crate::error::{BotError, BotResult};
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

pub const PURPOSE_TIMEOUT_MINUTES: i64 = 10;

pub async fn create_session<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    room_id: i32,
    host_user_id: u64,
) -> BotResult<rental_sessions::Model> {
    let now = Utc::now().fixed_offset();
    let deadline = (Utc::now() + Duration::minutes(PURPOSE_TIMEOUT_MINUTES)).fixed_offset();

    let model = rental_sessions::ActiveModel {
        guild_id: Set(guild_id as i64),
        room_id: Set(room_id),
        host_user_id: Set(host_user_id as i64),
        purpose: Set(None),
        state: Set(rental_sessions::STATE_AWAITING_PURPOSE),
        started_at: Set(now),
        purpose_deadline: Set(Some(deadline)),
        ended_at: Set(None),
        ..Default::default()
    };
    let session = model.insert(db).await?;

    let task = scheduled_tasks::ActiveModel {
        guild_id: Set(guild_id as i64),
        task_type: Set(TASK_TYPE_TIMEOUT_NOTIFICATION),
        rental_session_id: Set(Some(session.id)),
        schedule_datetime: Set(deadline),
        processed: Set(false),
        created_at: Set(now),
        ..Default::default()
    };
    task.insert(db).await?;

    Ok(session)
}

pub async fn create_active_session<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    room_id: i32,
    host_user_id: u64,
) -> BotResult<rental_sessions::Model> {
    let now = Utc::now().fixed_offset();

    let model = rental_sessions::ActiveModel {
        guild_id: Set(guild_id as i64),
        room_id: Set(room_id),
        host_user_id: Set(host_user_id as i64),
        purpose: Set(None),
        state: Set(STATE_ACTIVE),
        started_at: Set(now),
        purpose_deadline: Set(None),
        ended_at: Set(None),
        ..Default::default()
    };
    model.insert(db).await.map_err(BotError::from)
}

pub async fn set_purpose<C: ConnectionTrait>(
    db: &C,
    session_id: i32,
    purpose: String,
) -> BotResult<rental_sessions::Model> {
    let session = rental_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await?
        .ok_or_else(|| BotError::NotFound(format!("session {session_id}")))?;

    let mut model: rental_sessions::ActiveModel = session.into();
    model.purpose = Set(Some(purpose));
    model.state = Set(STATE_ACTIVE);
    model.purpose_deadline = Set(None);
    model.update(db).await.map_err(BotError::from)
}

/// Reassign a pending rental session to a different room. Used when the rental modal's
/// VC dropdown picks a room different from the one originally allocated.
pub async fn set_session_room<C: ConnectionTrait>(
    db: &C,
    session_id: i32,
    new_room_id: i32,
) -> BotResult<()> {
    let session = rental_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await?
        .ok_or_else(|| BotError::NotFound(format!("session {session_id}")))?;

    let mut model: rental_sessions::ActiveModel = session.into();
    model.room_id = Set(new_room_id);
    model.update(db).await?;
    Ok(())
}

pub async fn release_session<C: ConnectionTrait>(db: &C, session_id: i32) -> BotResult<()> {
    let session = rental_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await?
        .ok_or_else(|| BotError::NotFound(format!("session {session_id}")))?;

    let mut model: rental_sessions::ActiveModel = session.into();
    model.state = Set(STATE_RELEASED);
    model.ended_at = Set(Some(Utc::now().fixed_offset()));
    model.update(db).await?;
    Ok(())
}

pub async fn transfer_host<C: ConnectionTrait>(
    db: &C,
    session_id: i32,
    new_host_id: u64,
) -> BotResult<rental_sessions::Model> {
    let session = rental_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await?
        .ok_or_else(|| BotError::NotFound(format!("session {session_id}")))?;

    let mut model: rental_sessions::ActiveModel = session.into();
    model.host_user_id = Set(new_host_id as i64);
    model.state = Set(STATE_ACTIVE);
    model.update(db).await.map_err(BotError::from)
}

pub async fn set_pending_handoff<C: ConnectionTrait>(db: &C, session_id: i32) -> BotResult<()> {
    let session = rental_sessions::Entity::find_by_id(session_id)
        .one(db)
        .await?
        .ok_or_else(|| BotError::NotFound(format!("session {session_id}")))?;

    let mut model: rental_sessions::ActiveModel = session.into();
    model.state = Set(STATE_PENDING_HANDOFF);
    model.update(db).await?;
    Ok(())
}

pub async fn find_active_session_for_room<C: ConnectionTrait>(
    db: &C,
    room_id: i32,
) -> BotResult<Option<rental_sessions::Model>> {
    rental_sessions::Entity::find()
        .filter(rental_sessions::Column::RoomId.eq(room_id))
        .filter(rental_sessions::Column::State.is_in([
            rental_sessions::STATE_AWAITING_PURPOSE,
            STATE_ACTIVE,
            STATE_PENDING_HANDOFF,
        ]))
        .one(db)
        .await
        .map_err(BotError::from)
}

pub async fn find_active_session_for_user<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    user_id: u64,
) -> BotResult<Option<rental_sessions::Model>> {
    rental_sessions::Entity::find()
        .filter(rental_sessions::Column::GuildId.eq(guild_id as i64))
        .filter(rental_sessions::Column::HostUserId.eq(user_id as i64))
        .filter(
            rental_sessions::Column::State
                .is_in([rental_sessions::STATE_AWAITING_PURPOSE, STATE_ACTIVE]),
        )
        .one(db)
        .await
        .map_err(BotError::from)
}

pub async fn find_active_sessions_by_guild<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
) -> BotResult<Vec<rental_sessions::Model>> {
    rental_sessions::Entity::find()
        .filter(rental_sessions::Column::GuildId.eq(guild_id as i64))
        .filter(rental_sessions::Column::State.is_in([
            STATE_AWAITING_PURPOSE,
            STATE_ACTIVE,
            STATE_PENDING_HANDOFF,
        ]))
        .all(db)
        .await
        .map_err(BotError::from)
}

/// Mark all unprocessed scheduled tasks for a rental session as processed.
pub async fn mark_session_tasks_processed<C: ConnectionTrait>(
    db: &C,
    session_id: i32,
) -> BotResult<()> {
    let tasks = scheduled_tasks::Entity::find()
        .filter(scheduled_tasks::Column::RentalSessionId.eq(session_id))
        .filter(scheduled_tasks::Column::Processed.eq(false))
        .all(db)
        .await?;
    for task in tasks {
        let mut model: scheduled_tasks::ActiveModel = task.into();
        model.processed = Set(true);
        model.update(db).await?;
    }
    Ok(())
}

pub async fn mark_task_processed<C: ConnectionTrait>(db: &C, task_id: i32) -> BotResult<()> {
    let task = scheduled_tasks::Entity::find_by_id(task_id)
        .one(db)
        .await?
        .ok_or_else(|| BotError::NotFound(format!("task {task_id}")))?;
    let mut model: scheduled_tasks::ActiveModel = task.into();
    model.processed = Set(true);
    model.update(db).await?;
    Ok(())
}
