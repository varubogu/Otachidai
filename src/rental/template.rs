use std::collections::HashMap;
use std::fmt;

/// Mustache-flavoured `{{ name }}` template used for the rental purpose auto-post.
///
/// Supports the fixed variable vocabulary listed in `VarKey`. Literal `{{` / `}}` can be
/// emitted by escaping with a backslash (`\{{` / `\}}`). Anything else inside the braces
/// is treated as a variable reference; unknown references are rejected at validation time
/// so the rendered output never silently drops content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Literal(String),
    Var(VarKey),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarKey {
    User,
    Room,
    When,
    Preset,
    /// 1-based question index (q1..q10).
    Q(u8),
    /// `{{answer:質問名}}` — answer keyed by the question's literal text.
    Answer(String),
    /// Pre-assembled multi-line "Q + A" block.
    Answers,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateError {
    UnterminatedTag { byte_offset: usize },
    UnexpectedClose { byte_offset: usize },
    EmptyTag { byte_offset: usize },
    UnknownVariable { name: String },
    InvalidEscape { byte_offset: usize },
    UnsupportedQIndex(u8),
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemplateError::UnterminatedTag { byte_offset } => {
                write!(f, "unterminated `{{{{` at byte {byte_offset}")
            }
            TemplateError::UnexpectedClose { byte_offset } => {
                write!(f, "unexpected `}}}}` at byte {byte_offset}")
            }
            TemplateError::EmptyTag { byte_offset } => {
                write!(f, "empty `{{{{ }}}}` tag at byte {byte_offset}")
            }
            TemplateError::UnknownVariable { name } => {
                write!(f, "unknown variable `{{{{{name}}}}}`")
            }
            TemplateError::InvalidEscape { byte_offset } => {
                write!(f, "invalid escape sequence at byte {byte_offset}")
            }
            TemplateError::UnsupportedQIndex(n) => {
                write!(f, "question index `q{n}` out of range (1-10)")
            }
        }
    }
}

impl std::error::Error for TemplateError {}

pub fn parse(input: &str) -> Result<Template, TemplateError> {
    let mut segments: Vec<Segment> = Vec::new();
    let mut literal = String::new();
    let bytes = input.as_bytes();
    let mut i: usize = 0;

    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 2 <= bytes.len() {
            // Allow `\{{` and `\}}` to emit literal braces.
            let next = &input[i + 1..];
            if next.starts_with("{{") {
                literal.push_str("{{");
                i += 3;
                continue;
            }
            if next.starts_with("}}") {
                literal.push_str("}}");
                i += 3;
                continue;
            }
            // Lone backslashes are kept verbatim — they aren't meaningful unless paired
            // with a brace pair, and treating every `\` as an error would be hostile to
            // free-form text.
            literal.push('\\');
            i += 1;
            continue;
        }

        if input[i..].starts_with("{{") {
            if !literal.is_empty() {
                segments.push(Segment::Literal(std::mem::take(&mut literal)));
            }
            let tag_start = i + 2;
            let Some(rel_end) = input[tag_start..].find("}}") else {
                return Err(TemplateError::UnterminatedTag { byte_offset: i });
            };
            let raw_name = &input[tag_start..tag_start + rel_end];
            let trimmed = raw_name.trim();
            if trimmed.is_empty() {
                return Err(TemplateError::EmptyTag { byte_offset: i });
            }
            let key = parse_var_key(trimmed)?;
            segments.push(Segment::Var(key));
            i = tag_start + rel_end + 2;
            continue;
        }

        if input[i..].starts_with("}}") {
            return Err(TemplateError::UnexpectedClose { byte_offset: i });
        }

        // Push one char (handle multi-byte UTF-8 correctly).
        let ch = input[i..].chars().next().expect("non-empty by index check");
        literal.push(ch);
        i += ch.len_utf8();
    }

    if !literal.is_empty() {
        segments.push(Segment::Literal(literal));
    }

    Ok(Template { segments })
}

fn parse_var_key(name: &str) -> Result<VarKey, TemplateError> {
    match name {
        "user" => Ok(VarKey::User),
        "room" => Ok(VarKey::Room),
        "when" => Ok(VarKey::When),
        "preset" => Ok(VarKey::Preset),
        "answers" => Ok(VarKey::Answers),
        other => {
            if let Some(idx_str) = other.strip_prefix('q')
                && let Ok(idx) = idx_str.parse::<u8>()
            {
                if (1..=10).contains(&idx) {
                    return Ok(VarKey::Q(idx));
                }
                return Err(TemplateError::UnsupportedQIndex(idx));
            }
            if let Some(rest) = other.strip_prefix("answer:") {
                return Ok(VarKey::Answer(rest.trim().to_string()));
            }
            Err(TemplateError::UnknownVariable {
                name: other.to_string(),
            })
        }
    }
}

/// Catalogue of variables a template is allowed to reference.
///
/// Used at upload time to reject references the preset can't satisfy — e.g. `{{q3}}` on a
/// preset that defines only two questions, or `{{answer:存在しない}}`.
pub struct TemplateSchema<'a> {
    pub question_texts: &'a [String],
}

impl<'a> TemplateSchema<'a> {
    pub fn new(question_texts: &'a [String]) -> Self {
        Self { question_texts }
    }
}

pub fn validate(template: &Template, schema: &TemplateSchema<'_>) -> Result<(), TemplateError> {
    for seg in &template.segments {
        if let Segment::Var(key) = seg {
            match key {
                VarKey::Q(n) => {
                    if (*n as usize) > schema.question_texts.len() {
                        return Err(TemplateError::UnknownVariable {
                            name: format!("q{n}"),
                        });
                    }
                }
                VarKey::Answer(name) => {
                    if !schema.question_texts.iter().any(|t| t == name) {
                        return Err(TemplateError::UnknownVariable {
                            name: format!("answer:{name}"),
                        });
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

pub struct RenderContext {
    pub user: String,
    pub room: String,
    pub when: String,
    pub preset: String,
    /// Answers indexed 0..N (i.e. q1 == questions[0]).
    pub questions: Vec<String>,
    pub answers_by_name: HashMap<String, String>,
    pub answers_text: String,
}

pub fn render(template: &Template, ctx: &RenderContext) -> String {
    let mut out = String::new();
    for seg in &template.segments {
        match seg {
            Segment::Literal(s) => out.push_str(s),
            Segment::Var(key) => match key {
                VarKey::User => out.push_str(&ctx.user),
                VarKey::Room => out.push_str(&ctx.room),
                VarKey::When => out.push_str(&ctx.when),
                VarKey::Preset => out.push_str(&ctx.preset),
                VarKey::Answers => out.push_str(&ctx.answers_text),
                VarKey::Q(n) => {
                    let idx = (*n as usize).saturating_sub(1);
                    if let Some(val) = ctx.questions.get(idx) {
                        out.push_str(val);
                    }
                }
                VarKey::Answer(name) => {
                    if let Some(val) = ctx.answers_by_name.get(name) {
                        out.push_str(val);
                    }
                }
            },
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema(qs: &[&str]) -> Vec<String> {
        qs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_literal_only() {
        let t = parse("hello world").unwrap();
        assert_eq!(t.segments, vec![Segment::Literal("hello world".into())]);
    }

    #[test]
    fn parses_simple_var() {
        let t = parse("{{user}}").unwrap();
        assert_eq!(t.segments, vec![Segment::Var(VarKey::User)]);
    }

    #[test]
    fn parses_var_with_whitespace() {
        let t = parse("{{   user   }}").unwrap();
        assert_eq!(t.segments, vec![Segment::Var(VarKey::User)]);
    }

    #[test]
    fn parses_mixed() {
        let t = parse("hi {{user}}, room: {{room}}").unwrap();
        assert_eq!(
            t.segments,
            vec![
                Segment::Literal("hi ".into()),
                Segment::Var(VarKey::User),
                Segment::Literal(", room: ".into()),
                Segment::Var(VarKey::Room),
            ]
        );
    }

    #[test]
    fn parses_q_index() {
        let t = parse("{{q1}} / {{q10}}").unwrap();
        assert_eq!(
            t.segments,
            vec![
                Segment::Var(VarKey::Q(1)),
                Segment::Literal(" / ".into()),
                Segment::Var(VarKey::Q(10)),
            ]
        );
    }

    #[test]
    fn parses_answer_by_name() {
        let t = parse("{{answer:目的}}").unwrap();
        assert_eq!(
            t.segments,
            vec![Segment::Var(VarKey::Answer("目的".into()))]
        );
    }

    #[test]
    fn escapes_braces() {
        let t = parse("literal \\{{not a var\\}} done").unwrap();
        assert_eq!(
            t.segments,
            vec![Segment::Literal("literal {{not a var}} done".into())]
        );
    }

    #[test]
    fn rejects_unterminated() {
        assert!(matches!(
            parse("hi {{user"),
            Err(TemplateError::UnterminatedTag { .. })
        ));
    }

    #[test]
    fn rejects_unexpected_close() {
        assert!(matches!(
            parse("oops }} here"),
            Err(TemplateError::UnexpectedClose { .. })
        ));
    }

    #[test]
    fn rejects_empty_tag() {
        assert!(matches!(
            parse("{{  }}"),
            Err(TemplateError::EmptyTag { .. })
        ));
    }

    #[test]
    fn rejects_unknown_var() {
        assert!(matches!(
            parse("{{wat}}"),
            Err(TemplateError::UnknownVariable { .. })
        ));
    }

    #[test]
    fn rejects_q_out_of_range() {
        assert!(matches!(
            parse("{{q11}}"),
            Err(TemplateError::UnsupportedQIndex(11))
        ));
    }

    #[test]
    fn validate_q_out_of_preset_range() {
        let t = parse("{{q3}}").unwrap();
        let qs = schema(&["a", "b"]);
        let s = TemplateSchema::new(&qs);
        assert!(matches!(
            validate(&t, &s),
            Err(TemplateError::UnknownVariable { .. })
        ));
    }

    #[test]
    fn validate_q_in_preset_range() {
        let t = parse("{{q2}}").unwrap();
        let qs = schema(&["a", "b"]);
        let s = TemplateSchema::new(&qs);
        assert!(validate(&t, &s).is_ok());
    }

    #[test]
    fn validate_answer_by_name() {
        let t = parse("{{answer:目的}}").unwrap();
        let qs = schema(&["目的", "備考"]);
        let s = TemplateSchema::new(&qs);
        assert!(validate(&t, &s).is_ok());

        let bad = parse("{{answer:存在しない}}").unwrap();
        assert!(matches!(
            validate(&bad, &s),
            Err(TemplateError::UnknownVariable { .. })
        ));
    }

    #[test]
    fn renders_basic() {
        let t = parse("hi {{user}} ({{room}})").unwrap();
        let ctx = RenderContext {
            user: "<@1>".into(),
            room: "<#2>".into(),
            when: String::new(),
            preset: String::new(),
            questions: vec![],
            answers_by_name: HashMap::new(),
            answers_text: String::new(),
        };
        assert_eq!(render(&t, &ctx), "hi <@1> (<#2>)");
    }

    #[test]
    fn renders_q_and_answer_by_name() {
        let t = parse("{{q1}}/{{answer:目的}}").unwrap();
        let mut by_name = HashMap::new();
        by_name.insert("目的".to_string(), "雑談".to_string());
        let ctx = RenderContext {
            user: String::new(),
            room: String::new(),
            when: String::new(),
            preset: String::new(),
            questions: vec!["雑談".into()],
            answers_by_name: by_name,
            answers_text: String::new(),
        };
        assert_eq!(render(&t, &ctx), "雑談/雑談");
    }

    #[test]
    fn renders_missing_answer_as_empty() {
        let t = parse("[{{answer:備考}}]").unwrap();
        let ctx = RenderContext {
            user: String::new(),
            room: String::new(),
            when: String::new(),
            preset: String::new(),
            questions: vec![],
            answers_by_name: HashMap::new(),
            answers_text: String::new(),
        };
        assert_eq!(render(&t, &ctx), "[]");
    }
}
