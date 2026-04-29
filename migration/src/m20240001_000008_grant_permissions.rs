use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        // guild_master schema grants
        conn.execute_unprepared(
            "GRANT USAGE ON SCHEMA guild_master TO \
             otachidai_bot_guild, otachidai_bot_system, otachidai_bot_global, otachidai_bot_cleanup",
        )
        .await?;

        conn.execute_unprepared(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA guild_master \
             TO otachidai_bot_guild",
        )
        .await?;

        conn.execute_unprepared(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA guild_master \
             TO otachidai_bot_system",
        )
        .await?;

        conn.execute_unprepared(
            "GRANT ALL ON ALL TABLES IN SCHEMA guild_master TO otachidai_bot_global",
        )
        .await?;

        conn.execute_unprepared(
            "GRANT SELECT, DELETE ON ALL TABLES IN SCHEMA guild_master TO otachidai_bot_cleanup",
        )
        .await?;

        conn.execute_unprepared(
            "GRANT USAGE ON ALL SEQUENCES IN SCHEMA guild_master TO \
             otachidai_bot_guild, otachidai_bot_system, otachidai_bot_global",
        )
        .await?;

        // worker schema grants
        conn.execute_unprepared(
            "GRANT USAGE ON SCHEMA worker TO \
             otachidai_bot_guild, otachidai_bot_system, otachidai_bot_global, otachidai_bot_cleanup",
        )
        .await?;

        conn.execute_unprepared(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA worker \
             TO otachidai_bot_system",
        )
        .await?;

        conn.execute_unprepared(
            "GRANT SELECT, INSERT ON ALL TABLES IN SCHEMA worker TO otachidai_bot_guild",
        )
        .await?;

        conn.execute_unprepared(
            "GRANT SELECT, DELETE ON ALL TABLES IN SCHEMA worker TO otachidai_bot_cleanup",
        )
        .await?;

        conn.execute_unprepared(
            "GRANT USAGE ON ALL SEQUENCES IN SCHEMA worker TO \
             otachidai_bot_system, otachidai_bot_guild",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
