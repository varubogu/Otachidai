use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        conn.execute_unprepared(
            "CREATE TABLE guild_master.room_groups (
                id          SERIAL PRIMARY KEY,
                guild_id    BIGINT NOT NULL REFERENCES guild_master.guilds(guild_id) ON DELETE CASCADE,
                name        TEXT NOT NULL,
                channel_id  BIGINT NOT NULL,
                message_id  BIGINT,
                created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
                UNIQUE (guild_id, name)
            )",
        )
        .await?;

        conn.execute_unprepared(
            "ALTER TABLE guild_master.rooms
             ADD COLUMN group_id INT REFERENCES guild_master.room_groups(id) ON DELETE SET NULL",
        )
        .await?;

        conn.execute_unprepared(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE guild_master.room_groups
             TO otachidai_bot_guild, otachidai_bot_system",
        )
        .await?;

        conn.execute_unprepared(
            "GRANT USAGE ON SEQUENCE guild_master.room_groups_id_seq
             TO otachidai_bot_guild, otachidai_bot_system",
        )
        .await?;

        conn.execute_unprepared("ALTER TABLE guild_master.room_groups ENABLE ROW LEVEL SECURITY")
            .await?;

        conn.execute_unprepared(
            "CREATE POLICY guild_isolation ON guild_master.room_groups
             AS PERMISSIVE FOR ALL TO otachidai_bot_guild
             USING (guild_id = current_setting('app.current_guild_id', true)::BIGINT)",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        conn.execute_unprepared("ALTER TABLE guild_master.rooms DROP COLUMN IF EXISTS group_id")
            .await?;

        conn.execute_unprepared(
            "DROP POLICY IF EXISTS guild_isolation ON guild_master.room_groups",
        )
        .await?;

        conn.execute_unprepared("DROP TABLE IF EXISTS guild_master.room_groups")
            .await?;

        Ok(())
    }
}
