use crate::entities::rental_question_presets;
use crate::error::{BotError, BotResult};
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

pub async fn upsert_preset<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    name: String,
    questions: Vec<Option<String>>,
) -> BotResult<rental_question_presets::Model> {
    let now = chrono::Utc::now().fixed_offset();
    let existing = find_by_name(db, guild_id, &name).await?;
    let questions = normalize_questions(questions);

    if let Some(existing) = existing {
        let mut model: rental_question_presets::ActiveModel = existing.into();
        model.question_1 = Set(questions[0].clone());
        model.question_2 = Set(questions[1].clone());
        model.question_3 = Set(questions[2].clone());
        model.question_4 = Set(questions[3].clone());
        model.question_5 = Set(questions[4].clone());
        model.question_6 = Set(questions[5].clone());
        model.question_7 = Set(questions[6].clone());
        model.question_8 = Set(questions[7].clone());
        model.question_9 = Set(questions[8].clone());
        model.question_10 = Set(questions[9].clone());
        model.updated_at = Set(now);
        model.update(db).await.map_err(BotError::from)
    } else {
        let model = rental_question_presets::ActiveModel {
            guild_id: Set(guild_id as i64),
            name: Set(name),
            question_1: Set(questions[0].clone()),
            question_2: Set(questions[1].clone()),
            question_3: Set(questions[2].clone()),
            question_4: Set(questions[3].clone()),
            question_5: Set(questions[4].clone()),
            question_6: Set(questions[5].clone()),
            question_7: Set(questions[6].clone()),
            question_8: Set(questions[7].clone()),
            question_9: Set(questions[8].clone()),
            question_10: Set(questions[9].clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        model.insert(db).await.map_err(BotError::from)
    }
}

pub async fn find_by_name<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    name: &str,
) -> BotResult<Option<rental_question_presets::Model>> {
    rental_question_presets::Entity::find()
        .filter(rental_question_presets::Column::GuildId.eq(guild_id as i64))
        .filter(rental_question_presets::Column::Name.eq(name))
        .one(db)
        .await
        .map_err(BotError::from)
}

pub async fn find_by_id<C: ConnectionTrait>(
    db: &C,
    preset_id: i32,
) -> BotResult<Option<rental_question_presets::Model>> {
    rental_question_presets::Entity::find_by_id(preset_id)
        .one(db)
        .await
        .map_err(BotError::from)
}

fn normalize_questions(mut questions: Vec<Option<String>>) -> Vec<Option<String>> {
    questions.resize_with(10, || None);
    questions
        .into_iter()
        .take(10)
        .map(|question| question.and_then(|q| normalize_optional_text(&q)))
        .collect()
}

pub fn normalize_optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
