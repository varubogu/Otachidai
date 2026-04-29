use crate::entities::guild_channels::{self, CHANNEL_TYPE_RENTAL_BUTTON, CHANNEL_TYPE_REPORT};
use crate::entities::guilds;
use crate::error::{BotError, BotResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set,
    sea_query::OnConflict,
};

pub async fn ensure_guild<C: ConnectionTrait>(db: &C, guild_id: u64) -> BotResult<guilds::Model> {
    let id = guild_id as i64;
    if let Some(g) = guilds::Entity::find_by_id(id).one(db).await? {
        return Ok(g);
    }
    let now = chrono::Utc::now().fixed_offset();
    let model = guilds::ActiveModel {
        guild_id: Set(id),
        language: Set("en".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let insert_result = guilds::Entity::insert(model)
        .on_conflict(
            OnConflict::column(guilds::Column::GuildId)
                .do_nothing()
                .to_owned(),
        )
        .exec(db)
        .await;

    match insert_result {
        Ok(_) | Err(sea_orm::DbErr::RecordNotInserted) => {}
        Err(e) => return Err(BotError::Db(e)),
    }

    guilds::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| BotError::NotFound("guild".to_string()))
}

pub async fn get_language<C: ConnectionTrait>(db: &C, guild_id: u64) -> BotResult<String> {
    let guild = guilds::Entity::find_by_id(guild_id as i64).one(db).await?;
    Ok(guild
        .map(|g| g.language)
        .unwrap_or_else(|| "en".to_string()))
}

pub async fn set_report_channel<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    channel_id: u64,
) -> BotResult<()> {
    upsert_channel(db, guild_id, channel_id, CHANNEL_TYPE_REPORT).await
}

pub async fn set_rental_button_channel<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    channel_id: u64,
) -> BotResult<()> {
    upsert_channel(db, guild_id, channel_id, CHANNEL_TYPE_RENTAL_BUTTON).await
}

pub async fn get_report_channel<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
) -> BotResult<Option<i64>> {
    get_channel(db, guild_id, CHANNEL_TYPE_REPORT).await
}

pub async fn get_rental_button_channel<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
) -> BotResult<Option<i64>> {
    get_channel(db, guild_id, CHANNEL_TYPE_RENTAL_BUTTON).await
}

async fn get_channel<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    channel_type: i16,
) -> BotResult<Option<i64>> {
    let row = guild_channels::Entity::find()
        .filter(guild_channels::Column::GuildId.eq(guild_id as i64))
        .filter(guild_channels::Column::ChannelType.eq(channel_type))
        .one(db)
        .await?;
    Ok(row.map(|r| r.channel_id))
}

async fn upsert_channel<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    channel_id: u64,
    channel_type: i16,
) -> BotResult<()> {
    let existing = guild_channels::Entity::find()
        .filter(guild_channels::Column::GuildId.eq(guild_id as i64))
        .filter(guild_channels::Column::ChannelType.eq(channel_type))
        .one(db)
        .await?;

    if let Some(existing) = existing {
        let mut model: guild_channels::ActiveModel = existing.into();
        model.channel_id = Set(channel_id as i64);
        model.update(db).await?;
    } else {
        let model = guild_channels::ActiveModel {
            guild_id: Set(guild_id as i64),
            channel_id: Set(channel_id as i64),
            channel_type: Set(channel_type),
            created_at: Set(chrono::Utc::now().fixed_offset()),
            ..Default::default()
        };
        model.insert(db).await?;
    }
    Ok(())
}
