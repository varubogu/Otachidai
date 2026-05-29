use crate::entities::rental_routing_rules;
use crate::error::{BotError, BotResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

pub async fn find_rule<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    preset_id: i32,
    match_value: &str,
) -> BotResult<Option<rental_routing_rules::Model>> {
    rental_routing_rules::Entity::find()
        .filter(rental_routing_rules::Column::GuildId.eq(guild_id as i64))
        .filter(rental_routing_rules::Column::PresetId.eq(preset_id))
        .filter(rental_routing_rules::Column::MatchValue.eq(match_value))
        .one(db)
        .await
        .map_err(BotError::from)
}

pub async fn list_rules<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
) -> BotResult<Vec<rental_routing_rules::Model>> {
    rental_routing_rules::Entity::find()
        .filter(rental_routing_rules::Column::GuildId.eq(guild_id as i64))
        .order_by_asc(rental_routing_rules::Column::PresetId)
        .order_by_asc(rental_routing_rules::Column::Id)
        .all(db)
        .await
        .map_err(BotError::from)
}

pub async fn list_rules_for_preset<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    preset_id: i32,
) -> BotResult<Vec<rental_routing_rules::Model>> {
    rental_routing_rules::Entity::find()
        .filter(rental_routing_rules::Column::GuildId.eq(guild_id as i64))
        .filter(rental_routing_rules::Column::PresetId.eq(preset_id))
        .order_by_asc(rental_routing_rules::Column::Id)
        .all(db)
        .await
        .map_err(BotError::from)
}

pub async fn delete_all_for_guild<C: ConnectionTrait>(db: &C, guild_id: u64) -> BotResult<()> {
    rental_routing_rules::Entity::delete_many()
        .filter(rental_routing_rules::Column::GuildId.eq(guild_id as i64))
        .exec(db)
        .await?;
    Ok(())
}

pub struct RuleInput {
    pub preset_id: i32,
    pub match_value: String,
    pub channel_id: i64,
    pub template: Option<String>,
}

pub async fn insert_rule<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    rule: RuleInput,
) -> BotResult<rental_routing_rules::Model> {
    let now = chrono::Utc::now().fixed_offset();
    let model = rental_routing_rules::ActiveModel {
        guild_id: Set(guild_id as i64),
        preset_id: Set(rule.preset_id),
        match_value: Set(rule.match_value),
        channel_id: Set(rule.channel_id),
        template: Set(rule.template),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    model.insert(db).await.map_err(BotError::from)
}
