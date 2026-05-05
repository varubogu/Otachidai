use crate::entities::rooms;
use crate::error::{BotError, BotResult};
use crate::facade::rental as rental_facade;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

pub async fn register_room<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    text_channel_id: Option<u64>,
    voice_channel_id: Option<u64>,
) -> BotResult<rooms::Model> {
    let model = rooms::ActiveModel {
        guild_id: Set(guild_id as i64),
        text_channel_id: Set(text_channel_id.map(|id| id as i64)),
        voice_channel_id: Set(voice_channel_id.map(|id| id as i64)),
        is_available: Set(true),
        created_at: Set(chrono::Utc::now().fixed_offset()),
        ..Default::default()
    };
    model.insert(db).await.map_err(BotError::from)
}

pub async fn delete_room<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    text_channel_id: Option<u64>,
    voice_channel_id: Option<u64>,
) -> BotResult<bool> {
    let mut query = rooms::Entity::find().filter(rooms::Column::GuildId.eq(guild_id as i64));

    if let Some(tid) = text_channel_id {
        query = query.filter(rooms::Column::TextChannelId.eq(tid as i64));
    }
    if let Some(vid) = voice_channel_id {
        query = query.filter(rooms::Column::VoiceChannelId.eq(vid as i64));
    }

    let room = query.one(db).await?;
    match room {
        Some(r) => {
            rooms::Entity::delete_by_id(r.id).exec(db).await?;
            Ok(true)
        }
        None => Ok(false),
    }
}

pub async fn find_available_room<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
) -> BotResult<Option<rooms::Model>> {
    let rooms = rooms::Entity::find()
        .filter(rooms::Column::GuildId.eq(guild_id as i64))
        .order_by_asc(rooms::Column::Id)
        .all(db)
        .await?;

    for room in rooms {
        let active_session = rental_facade::find_active_session_for_room(db, room.id).await?;
        if active_session.is_none() {
            return Ok(Some(room));
        }
    }

    Ok(None)
}

pub async fn find_room_by_voice_channel<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    voice_channel_id: u64,
) -> BotResult<Option<rooms::Model>> {
    rooms::Entity::find()
        .filter(rooms::Column::GuildId.eq(guild_id as i64))
        .filter(rooms::Column::VoiceChannelId.eq(voice_channel_id as i64))
        .one(db)
        .await
        .map_err(BotError::from)
}

pub async fn set_room_availability<C: ConnectionTrait>(
    db: &C,
    room_id: i32,
    available: bool,
) -> BotResult<()> {
    let room = rooms::Entity::find_by_id(room_id)
        .one(db)
        .await?
        .ok_or_else(|| BotError::NotFound(format!("room {room_id}")))?;
    let mut model: rooms::ActiveModel = room.into();
    model.is_available = Set(available);
    model.update(db).await?;
    Ok(())
}
