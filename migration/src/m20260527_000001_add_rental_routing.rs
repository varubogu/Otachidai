use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        conn.execute_unprepared(
            "ALTER TABLE guild_master.rental_question_presets
             ADD COLUMN routing_key_index SMALLINT",
        )
        .await?;

        conn.execute_unprepared(
            "ALTER TABLE guild_master.rental_question_presets
             ADD CONSTRAINT rental_question_presets_routing_key_index_range
             CHECK (routing_key_index IS NULL OR (routing_key_index >= 0 AND routing_key_index <= 9))",
        )
        .await?;

        conn.execute_unprepared(
            "ALTER TABLE guild_master.guild_channels
             ADD COLUMN template TEXT",
        )
        .await?;

        conn.execute_unprepared(
            "CREATE TABLE guild_master.rental_routing_rules (
                id          SERIAL PRIMARY KEY,
                guild_id    BIGINT NOT NULL REFERENCES guild_master.guilds(guild_id) ON DELETE CASCADE,
                preset_id   INT NOT NULL REFERENCES guild_master.rental_question_presets(id) ON DELETE CASCADE,
                match_value TEXT NOT NULL,
                channel_id  BIGINT NOT NULL,
                template    TEXT,
                created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
                UNIQUE (guild_id, preset_id, match_value)
            )",
        )
        .await?;

        conn.execute_unprepared(
            "CREATE INDEX rental_routing_rules_guild_preset_idx
             ON guild_master.rental_routing_rules (guild_id, preset_id)",
        )
        .await?;

        conn.execute_unprepared(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON TABLE guild_master.rental_routing_rules
             TO otachidai_bot_guild, otachidai_bot_system",
        )
        .await?;

        conn.execute_unprepared(
            "GRANT USAGE ON SEQUENCE guild_master.rental_routing_rules_id_seq
             TO otachidai_bot_guild, otachidai_bot_system",
        )
        .await?;

        conn.execute_unprepared(
            "ALTER TABLE guild_master.rental_routing_rules ENABLE ROW LEVEL SECURITY",
        )
        .await?;

        conn.execute_unprepared(
            "CREATE POLICY guild_isolation ON guild_master.rental_routing_rules
             AS PERMISSIVE FOR ALL TO otachidai_bot_guild
             USING (guild_id = current_setting('app.current_guild_id', true)::BIGINT)",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        conn.execute_unprepared(
            "DROP POLICY IF EXISTS guild_isolation ON guild_master.rental_routing_rules",
        )
        .await?;

        conn.execute_unprepared("DROP TABLE IF EXISTS guild_master.rental_routing_rules")
            .await?;

        conn.execute_unprepared(
            "ALTER TABLE guild_master.guild_channels DROP COLUMN IF EXISTS template",
        )
        .await?;

        conn.execute_unprepared(
            "ALTER TABLE guild_master.rental_question_presets
             DROP CONSTRAINT IF EXISTS rental_question_presets_routing_key_index_range",
        )
        .await?;

        conn.execute_unprepared(
            "ALTER TABLE guild_master.rental_question_presets DROP COLUMN IF EXISTS routing_key_index",
        )
        .await?;

        Ok(())
    }
}
