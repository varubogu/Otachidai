use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared("CREATE SCHEMA IF NOT EXISTS guild_master")
            .await?;
        conn.execute_unprepared("CREATE SCHEMA IF NOT EXISTS worker")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared("DROP SCHEMA IF EXISTS worker CASCADE")
            .await?;
        conn.execute_unprepared("DROP SCHEMA IF EXISTS guild_master CASCADE")
            .await?;
        Ok(())
    }
}
