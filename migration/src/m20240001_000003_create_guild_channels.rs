use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE guild_master.guild_channels (
                    id           SERIAL PRIMARY KEY,
                    guild_id     BIGINT NOT NULL REFERENCES guild_master.guilds(guild_id) ON DELETE CASCADE,
                    channel_id   BIGINT NOT NULL,
                    channel_type SMALLINT NOT NULL,
                    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
                    UNIQUE (guild_id, channel_type)
                )",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS guild_master.guild_channels")
            .await?;
        Ok(())
    }
}
