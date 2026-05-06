use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        conn.execute_unprepared(
            "CREATE TABLE guild_master.rental_question_presets (
                id          SERIAL PRIMARY KEY,
                guild_id    BIGINT NOT NULL REFERENCES guild_master.guilds(guild_id) ON DELETE CASCADE,
                name        TEXT NOT NULL,
                question_1  TEXT,
                question_2  TEXT,
                question_3  TEXT,
                question_4  TEXT,
                question_5  TEXT,
                question_6  TEXT,
                question_7  TEXT,
                question_8  TEXT,
                question_9  TEXT,
                question_10 TEXT,
                created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
                UNIQUE (guild_id, name)
            )",
        )
        .await?;

        conn.execute_unprepared(
            "ALTER TABLE guild_master.rooms
             ADD COLUMN question_preset_id INT REFERENCES guild_master.rental_question_presets(id) ON DELETE SET NULL",
        )
        .await?;

        conn.execute_unprepared(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE guild_master.rental_question_presets
             TO otachidai_bot_guild, otachidai_bot_system",
        )
        .await?;

        conn.execute_unprepared(
            "GRANT USAGE ON SEQUENCE guild_master.rental_question_presets_id_seq
             TO otachidai_bot_guild, otachidai_bot_system",
        )
        .await?;

        conn.execute_unprepared(
            "ALTER TABLE guild_master.rental_question_presets ENABLE ROW LEVEL SECURITY",
        )
        .await?;

        conn.execute_unprepared(
            "CREATE POLICY guild_isolation ON guild_master.rental_question_presets
             AS PERMISSIVE FOR ALL TO otachidai_bot_guild
             USING (guild_id = current_setting('app.current_guild_id', true)::BIGINT)",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        conn.execute_unprepared(
            "ALTER TABLE guild_master.rooms DROP COLUMN IF EXISTS question_preset_id",
        )
        .await?;

        conn.execute_unprepared(
            "DROP POLICY IF EXISTS guild_isolation ON guild_master.rental_question_presets",
        )
        .await?;

        conn.execute_unprepared("DROP TABLE IF EXISTS guild_master.rental_question_presets")
            .await?;

        Ok(())
    }
}
