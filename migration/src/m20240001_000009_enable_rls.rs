use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const TABLES: &[&str] = &[
    "guild_master.guilds",
    "guild_master.guild_channels",
    "guild_master.rooms",
    "guild_master.rental_sessions",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        for table in TABLES {
            conn.execute_unprepared(&format!("ALTER TABLE {table} ENABLE ROW LEVEL SECURITY"))
                .await?;

            conn.execute_unprepared(&format!(
                "CREATE POLICY guild_isolation ON {table} \
                 AS PERMISSIVE FOR ALL TO otachidai_bot_guild \
                 USING (guild_id = current_setting('app.current_guild_id', true)::BIGINT)"
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        for table in TABLES {
            conn.execute_unprepared(&format!("DROP POLICY IF EXISTS guild_isolation ON {table}"))
                .await?;

            conn.execute_unprepared(&format!("ALTER TABLE {table} DISABLE ROW LEVEL SECURITY"))
                .await?;
        }

        Ok(())
    }
}
