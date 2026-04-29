use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE guild_master.rental_sessions (
                    id               SERIAL PRIMARY KEY,
                    guild_id         BIGINT NOT NULL REFERENCES guild_master.guilds(guild_id),
                    room_id          INT NOT NULL REFERENCES guild_master.rooms(id) ON DELETE CASCADE,
                    host_user_id     BIGINT NOT NULL,
                    purpose          TEXT,
                    state            SMALLINT NOT NULL DEFAULT 1,
                    started_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
                    purpose_deadline TIMESTAMPTZ,
                    ended_at         TIMESTAMPTZ
                )",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS guild_master.rental_sessions")
            .await?;
        Ok(())
    }
}
