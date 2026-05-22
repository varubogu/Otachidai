use crate::entities::room_groups;
use crate::error::{BotError, BotResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

pub async fn register_group<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    name: &str,
    channel_id: u64,
) -> BotResult<room_groups::Model> {
    let model = room_groups::ActiveModel {
        guild_id: Set(guild_id as i64),
        name: Set(name.to_string()),
        channel_id: Set(channel_id as i64),
        message_id: Set(None),
        created_at: Set(chrono::Utc::now().fixed_offset()),
        ..Default::default()
    };
    model.insert(db).await.map_err(BotError::from)
}

pub async fn find_group_by_name<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    name: &str,
) -> BotResult<Option<room_groups::Model>> {
    room_groups::Entity::find()
        .filter(room_groups::Column::GuildId.eq(guild_id as i64))
        .filter(room_groups::Column::Name.eq(name))
        .one(db)
        .await
        .map_err(BotError::from)
}

pub async fn delete_group<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    name: &str,
) -> BotResult<Option<room_groups::Model>> {
    let group = find_group_by_name(db, guild_id, name).await?;
    if let Some(ref g) = group {
        room_groups::Entity::delete_by_id(g.id).exec(db).await?;
    }
    Ok(group)
}

pub async fn list_groups<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
) -> BotResult<Vec<room_groups::Model>> {
    room_groups::Entity::find()
        .filter(room_groups::Column::GuildId.eq(guild_id as i64))
        .order_by_asc(room_groups::Column::Id)
        .all(db)
        .await
        .map_err(BotError::from)
}

pub async fn set_group_message_id<C: ConnectionTrait>(
    db: &C,
    group_id: i32,
    message_id: Option<u64>,
) -> BotResult<()> {
    let group = room_groups::Entity::find_by_id(group_id)
        .one(db)
        .await?
        .ok_or_else(|| BotError::NotFound(format!("room_group {group_id}")))?;
    let mut model: room_groups::ActiveModel = group.into();
    model.message_id = Set(message_id.map(|id| id as i64));
    model.update(db).await?;
    Ok(())
}
