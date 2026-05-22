use crate::entities::rental_question_presets;
use crate::error::{BotError, BotResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

pub struct QuestionWithInput {
    pub index: usize,
    pub text: String,
    pub input: QuestionInput,
}

pub enum QuestionInput {
    Text,
    Dropdown(Vec<String>),
}

/// Parse comma-separated options, treating `,,` as an escaped literal comma.
/// Example: `"abc,def,,ghi"` → `["abc", "def,ghi"]`
pub fn parse_answer_options(raw: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == ',' {
            if chars.peek() == Some(&',') {
                chars.next();
                current.push(',');
            } else {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    result.push(trimmed);
                }
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        result.push(trimmed);
    }
    result
}

pub fn model_questions_with_inputs(
    model: &rental_question_presets::Model,
) -> Vec<QuestionWithInput> {
    let questions: [Option<&str>; 10] = [
        model.question_1.as_deref(),
        model.question_2.as_deref(),
        model.question_3.as_deref(),
        model.question_4.as_deref(),
        model.question_5.as_deref(),
        model.question_6.as_deref(),
        model.question_7.as_deref(),
        model.question_8.as_deref(),
        model.question_9.as_deref(),
        model.question_10.as_deref(),
    ];
    let answers: [Option<&str>; 10] = [
        model.answer_1.as_deref(),
        model.answer_2.as_deref(),
        model.answer_3.as_deref(),
        model.answer_4.as_deref(),
        model.answer_5.as_deref(),
        model.answer_6.as_deref(),
        model.answer_7.as_deref(),
        model.answer_8.as_deref(),
        model.answer_9.as_deref(),
        model.answer_10.as_deref(),
    ];

    questions
        .into_iter()
        .zip(answers.into_iter())
        .enumerate()
        .filter_map(|(i, (q_opt, a_opt))| {
            let q_text = q_opt?.trim();
            if q_text.is_empty() {
                return None;
            }
            let input = match a_opt {
                Some(raw) if !raw.trim().is_empty() => {
                    let opts = parse_answer_options(raw);
                    if opts.is_empty() {
                        QuestionInput::Text
                    } else {
                        QuestionInput::Dropdown(opts)
                    }
                }
                _ => QuestionInput::Text,
            };
            Some(QuestionWithInput {
                index: i,
                text: q_text.to_string(),
                input,
            })
        })
        .collect()
}

/// Assemble a human-readable purpose string from questions, dropdown answers (from state),
/// and text answers (from modal).
pub fn assemble_purpose(
    questions: &[QuestionWithInput],
    dropdown_answers: &[Option<String>],
    text_answers: &std::collections::HashMap<usize, String>,
    answer_prefix: &str,
) -> String {
    questions
        .iter()
        .enumerate()
        .map(|(display_idx, q)| {
            let answer = match &q.input {
                QuestionInput::Dropdown(_) => dropdown_answers
                    .get(q.index)
                    .and_then(|a| a.as_deref())
                    .unwrap_or("")
                    .to_string(),
                QuestionInput::Text => text_answers.get(&q.index).cloned().unwrap_or_default(),
            };
            format!(
                "{}. {}\n{}: {}",
                display_idx + 1,
                q.text,
                answer_prefix,
                answer
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub async fn upsert_preset<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    name: String,
    questions: Vec<Option<String>>,
    answers: Vec<Option<String>>,
) -> BotResult<rental_question_presets::Model> {
    let now = chrono::Utc::now().fixed_offset();
    let existing = find_by_name(db, guild_id, &name).await?;
    let questions = normalize_items(questions);
    let answers = normalize_items(answers);

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
        model.answer_1 = Set(answers[0].clone());
        model.answer_2 = Set(answers[1].clone());
        model.answer_3 = Set(answers[2].clone());
        model.answer_4 = Set(answers[3].clone());
        model.answer_5 = Set(answers[4].clone());
        model.answer_6 = Set(answers[5].clone());
        model.answer_7 = Set(answers[6].clone());
        model.answer_8 = Set(answers[7].clone());
        model.answer_9 = Set(answers[8].clone());
        model.answer_10 = Set(answers[9].clone());
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
            answer_1: Set(answers[0].clone()),
            answer_2: Set(answers[1].clone()),
            answer_3: Set(answers[2].clone()),
            answer_4: Set(answers[3].clone()),
            answer_5: Set(answers[4].clone()),
            answer_6: Set(answers[5].clone()),
            answer_7: Set(answers[6].clone()),
            answer_8: Set(answers[7].clone()),
            answer_9: Set(answers[8].clone()),
            answer_10: Set(answers[9].clone()),
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

/// List every preset for a guild, ordered by id (used for autocomplete suggestions).
pub async fn list_by_guild<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
) -> BotResult<Vec<rental_question_presets::Model>> {
    rental_question_presets::Entity::find()
        .filter(rental_question_presets::Column::GuildId.eq(guild_id as i64))
        .order_by_asc(rental_question_presets::Column::Id)
        .all(db)
        .await
        .map_err(BotError::from)
}

/// Resolve a preset reference accepted from a command option.
///
/// Suggestions are surfaced as `"id:name"`, so the submitted value is parsed by its
/// leading numeric id first; if that does not resolve to a preset in this guild we fall
/// back to treating the whole input as a preset name.
pub async fn find_by_ref<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    input: &str,
) -> BotResult<Option<rental_question_presets::Model>> {
    if let Some((id_part, _)) = input.split_once(':')
        && let Ok(id) = id_part.trim().parse::<i32>()
        && let Some(model) = find_by_id(db, id).await?
        && model.guild_id == guild_id as i64
    {
        return Ok(Some(model));
    }
    find_by_name(db, guild_id, input).await
}

/// Format a preset as the `"id:name"` label shown in autocomplete suggestions.
pub fn format_ref_label(model: &rental_question_presets::Model) -> String {
    format!("{}:{}", model.id, model.name)
}

fn normalize_items(mut items: Vec<Option<String>>) -> Vec<Option<String>> {
    items.resize_with(10, || None);
    items
        .into_iter()
        .take(10)
        .map(|item| item.and_then(|s| normalize_optional_text(&s)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_answer_options_basic() {
        assert_eq!(
            parse_answer_options("abc,def,ghi"),
            vec!["abc", "def", "ghi"]
        );
    }

    #[test]
    fn parse_answer_options_escaped_comma() {
        assert_eq!(parse_answer_options("abc,def,,ghi"), vec!["abc", "def,ghi"]);
    }

    #[test]
    fn parse_answer_options_leading_trailing_comma_escape() {
        assert_eq!(parse_answer_options("a,,b"), vec!["a,b"]);
    }

    #[test]
    fn parse_answer_options_empty() {
        assert_eq!(parse_answer_options(""), Vec::<String>::new());
    }

    #[test]
    fn parse_answer_options_single() {
        assert_eq!(parse_answer_options("only"), vec!["only"]);
    }

    #[test]
    fn parse_answer_options_trims_whitespace() {
        assert_eq!(parse_answer_options(" a , b , c "), vec!["a", "b", "c"]);
    }
}
