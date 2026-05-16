pub use sea_orm_migration::prelude::*;

mod m20240001_000001_create_schemas;
mod m20240001_000002_create_guilds;
mod m20240001_000003_create_guild_channels;
mod m20240001_000004_create_rooms;
mod m20240001_000005_create_rental_sessions;
mod m20240001_000006_create_scheduled_tasks;
mod m20240001_000007_create_notifications;
mod m20240001_000008_grant_permissions;
mod m20240001_000009_enable_rls;
mod m20240001_000010_default_language_ja;
mod m20260505_000001_add_rental_question_presets;
mod m20260516_000001_add_question_answers;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240001_000001_create_schemas::Migration),
            Box::new(m20240001_000002_create_guilds::Migration),
            Box::new(m20240001_000003_create_guild_channels::Migration),
            Box::new(m20240001_000004_create_rooms::Migration),
            Box::new(m20240001_000005_create_rental_sessions::Migration),
            Box::new(m20240001_000006_create_scheduled_tasks::Migration),
            Box::new(m20240001_000007_create_notifications::Migration),
            Box::new(m20240001_000008_grant_permissions::Migration),
            Box::new(m20240001_000009_enable_rls::Migration),
            Box::new(m20240001_000010_default_language_ja::Migration),
            Box::new(m20260505_000001_add_rental_question_presets::Migration),
            Box::new(m20260516_000001_add_question_answers::Migration),
        ]
    }
}
