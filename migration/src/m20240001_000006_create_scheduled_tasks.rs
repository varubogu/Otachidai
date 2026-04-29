use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE worker.scheduled_tasks (
                    id                SERIAL PRIMARY KEY,
                    guild_id          BIGINT NOT NULL,
                    task_type         SMALLINT NOT NULL,
                    rental_session_id INT REFERENCES guild_master.rental_sessions(id) ON DELETE CASCADE,
                    schedule_datetime TIMESTAMPTZ NOT NULL,
                    processed         BOOLEAN NOT NULL DEFAULT FALSE,
                    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
                )",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS worker.scheduled_tasks")
            .await?;
        Ok(())
    }
}
