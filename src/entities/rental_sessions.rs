use sea_orm::entity::prelude::*;

/// state: 1 = awaiting_purpose, 2 = active, 3 = released, 4 = pending_handoff
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "rental_sessions", schema_name = "guild_master")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub guild_id: i64,
    pub room_id: i32,
    pub host_user_id: i64,
    pub purpose: Option<String>,
    pub state: i16,
    pub started_at: DateTimeWithTimeZone,
    pub purpose_deadline: Option<DateTimeWithTimeZone>,
    pub ended_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub const STATE_AWAITING_PURPOSE: i16 = 1;
pub const STATE_ACTIVE: i16 = 2;
pub const STATE_RELEASED: i16 = 3;
pub const STATE_PENDING_HANDOFF: i16 = 4;
