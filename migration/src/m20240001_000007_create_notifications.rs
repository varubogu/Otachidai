use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE worker.notifications (
                    id                SERIAL PRIMARY KEY,
                    task_id           INT NOT NULL REFERENCES worker.scheduled_tasks(id) ON DELETE CASCADE,
                    guild_id          BIGINT NOT NULL,
                    schedule_datetime TIMESTAMPTZ NOT NULL,
                    sent              BOOLEAN NOT NULL DEFAULT FALSE,
                    sent_at           TIMESTAMPTZ,
                    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
                )",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS worker.notifications")
            .await?;
        Ok(())
    }
}
