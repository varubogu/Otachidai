use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "rental_question_presets", schema_name = "guild_master")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub guild_id: i64,
    pub name: String,
    pub question_1: Option<String>,
    pub question_2: Option<String>,
    pub question_3: Option<String>,
    pub question_4: Option<String>,
    pub question_5: Option<String>,
    pub question_6: Option<String>,
    pub question_7: Option<String>,
    pub question_8: Option<String>,
    pub question_9: Option<String>,
    pub question_10: Option<String>,
    pub answer_1: Option<String>,
    pub answer_2: Option<String>,
    pub answer_3: Option<String>,
    pub answer_4: Option<String>,
    pub answer_5: Option<String>,
    pub answer_6: Option<String>,
    pub answer_7: Option<String>,
    pub answer_8: Option<String>,
    pub answer_9: Option<String>,
    pub answer_10: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn questions(&self) -> Vec<String> {
        [
            &self.question_1,
            &self.question_2,
            &self.question_3,
            &self.question_4,
            &self.question_5,
            &self.question_6,
            &self.question_7,
            &self.question_8,
            &self.question_9,
            &self.question_10,
        ]
        .into_iter()
        .filter_map(|question| question.as_ref())
        .filter(|question| !question.trim().is_empty())
        .cloned()
        .collect()
    }
}
