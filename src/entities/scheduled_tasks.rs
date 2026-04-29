use sea_orm::entity::prelude::*;

/// task_type: 1 = timeout_notification
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "scheduled_tasks", schema_name = "worker")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub guild_id: i64,
    pub task_type: i16,
    pub rental_session_id: Option<i32>,
    pub schedule_datetime: DateTimeWithTimeZone,
    pub processed: bool,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub const TASK_TYPE_TIMEOUT_NOTIFICATION: i16 = 1;
