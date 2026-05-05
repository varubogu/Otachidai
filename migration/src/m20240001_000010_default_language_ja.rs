use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared("ALTER TABLE guild_master.guilds ALTER COLUMN language SET DEFAULT 'ja'")
            .await?;
        conn.execute_unprepared("UPDATE guild_master.guilds SET language = 'ja' WHERE language = 'en'")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared("ALTER TABLE guild_master.guilds ALTER COLUMN language SET DEFAULT 'en'")
            .await?;
        conn.execute_unprepared("UPDATE guild_master.guilds SET language = 'en' WHERE language = 'ja'")
            .await?;
        Ok(())
    }
}
