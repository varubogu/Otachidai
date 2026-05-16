use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            "ALTER TABLE guild_master.rental_question_presets
             ADD COLUMN answer_1  TEXT,
             ADD COLUMN answer_2  TEXT,
             ADD COLUMN answer_3  TEXT,
             ADD COLUMN answer_4  TEXT,
             ADD COLUMN answer_5  TEXT,
             ADD COLUMN answer_6  TEXT,
             ADD COLUMN answer_7  TEXT,
             ADD COLUMN answer_8  TEXT,
             ADD COLUMN answer_9  TEXT,
             ADD COLUMN answer_10 TEXT",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            "ALTER TABLE guild_master.rental_question_presets
             DROP COLUMN IF EXISTS answer_1,
             DROP COLUMN IF EXISTS answer_2,
             DROP COLUMN IF EXISTS answer_3,
             DROP COLUMN IF EXISTS answer_4,
             DROP COLUMN IF EXISTS answer_5,
             DROP COLUMN IF EXISTS answer_6,
             DROP COLUMN IF EXISTS answer_7,
             DROP COLUMN IF EXISTS answer_8,
             DROP COLUMN IF EXISTS answer_9,
             DROP COLUMN IF EXISTS answer_10",
        )
        .await?;
        Ok(())
    }
}
