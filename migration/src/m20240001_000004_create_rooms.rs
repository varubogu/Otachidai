use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE guild_master.rooms (
                    id               SERIAL PRIMARY KEY,
                    guild_id         BIGINT NOT NULL REFERENCES guild_master.guilds(guild_id) ON DELETE CASCADE,
                    text_channel_id  BIGINT,
                    voice_channel_id BIGINT,
                    is_available     BOOLEAN NOT NULL DEFAULT TRUE,
                    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
                    UNIQUE (guild_id, text_channel_id),
                    UNIQUE (guild_id, voice_channel_id)
                )",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS guild_master.rooms")
            .await?;
        Ok(())
    }
}
