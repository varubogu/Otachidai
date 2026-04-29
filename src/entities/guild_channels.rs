use sea_orm::entity::prelude::*;

/// channel_type: 1 = report channel, 2 = rental button channel
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "guild_channels", schema_name = "guild_master")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub guild_id: i64,
    pub channel_id: i64,
    pub channel_type: i16,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::guilds::Entity",
        from = "Column::GuildId",
        to = "super::guilds::Column::GuildId"
    )]
    Guild,
}

impl Related<super::guilds::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Guild.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub const CHANNEL_TYPE_REPORT: i16 = 1;
pub const CHANNEL_TYPE_RENTAL_BUTTON: i16 = 2;
