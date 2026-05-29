//! YAML 一括設定の解析・検証・適用・ダンプ。
//!
//! YAML はギルド設定全体のソース・オブ・トゥルース。アップロード時はすべての対象テーブルを
//! 全置換する（DELETE → INSERT）。アクティブセッションが乗っている部屋を削除しようとした
//! 場合の強制 release はアプリ層 (`rental::routing::force_release_for_rooms` 等) が担当し、
//! ここでは DB 側の整合だけを面倒見る。

use crate::entities::{
    guild_channels::{
        self, CHANNEL_TYPE_RENTAL_BUTTON, CHANNEL_TYPE_RENTAL_POST_FALLBACK, CHANNEL_TYPE_REPORT,
        CHANNEL_TYPE_ROOM_LIST,
    },
    guilds, rental_question_presets, rental_routing_rules, room_groups, rooms,
};
use crate::error::{BotError, BotResult};
use crate::facade::question_preset as qp_facade;
use crate::rental::template::{self, TemplateSchema};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const SUPPORTED_VERSION: u32 = 1;
const MAX_QUESTIONS_PER_PRESET: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildConfig {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guild: Option<GuildSection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channels: Option<ChannelsSection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub question_presets: Vec<PresetSection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub room_groups: Vec<RoomGroupSection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rooms: Vec<RoomSection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub routing_rules: Vec<PresetRoutingSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelsSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rental_button: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_list: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rental_post_fallback: Option<FallbackChannel>,
}

/// Either a bare channel id string or an object with channel + template.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FallbackChannel {
    Id(String),
    Object {
        channel: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        template: Option<String>,
    },
}

impl FallbackChannel {
    pub fn channel(&self) -> &str {
        match self {
            FallbackChannel::Id(s) => s,
            FallbackChannel::Object { channel, .. } => channel,
        }
    }
    pub fn template(&self) -> Option<&str> {
        match self {
            FallbackChannel::Id(_) => None,
            FallbackChannel::Object { template, .. } => template.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetSection {
    pub name: String,
    #[serde(default)]
    pub questions: Vec<QuestionSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionSection {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answers: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub routing_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomGroupSection {
    pub name: String,
    pub channel_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSection {
    pub voice_channel_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_channel_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question_preset: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetRoutingSection {
    pub preset: String,
    #[serde(default)]
    pub rules: Vec<RoutingRuleSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRuleSection {
    pub when: String,
    pub channel: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Validated config, ready to apply.
#[derive(Debug, Clone)]
pub struct ValidatedConfig {
    pub language: Option<String>,
    pub channels: ValidatedChannels,
    pub presets: Vec<ValidatedPreset>,
    pub groups: Vec<ValidatedGroup>,
    pub rooms: Vec<ValidatedRoom>,
    pub routing: Vec<ValidatedRouting>,
}

#[derive(Debug, Clone, Default)]
pub struct ValidatedChannels {
    pub report: Option<i64>,
    pub rental_button: Option<i64>,
    pub room_list: Option<i64>,
    pub rental_post_fallback: Option<(i64, Option<String>)>,
}

#[derive(Debug, Clone)]
pub struct ValidatedPreset {
    pub name: String,
    /// Length 10. Slot N (0-based) corresponds to question_{N+1} in DB.
    pub questions: Vec<Option<String>>,
    pub answers: Vec<Option<String>>,
    /// 0..=9 if the preset has a routing-key question.
    pub routing_key_index: Option<i16>,
}

#[derive(Debug, Clone)]
pub struct ValidatedGroup {
    pub name: String,
    pub channel_id: i64,
}

#[derive(Debug, Clone)]
pub struct ValidatedRoom {
    pub voice_channel_id: i64,
    pub text_channel_id: Option<i64>,
    pub group_name: Option<String>,
    pub preset_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ValidatedRouting {
    pub preset_name: String,
    pub match_value: String,
    pub channel_id: i64,
    pub template: Option<String>,
}

#[derive(Debug)]
pub enum ConfigError {
    Yaml(String),
    Validation(Vec<String>),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Yaml(s) => write!(f, "YAML parse error: {s}"),
            ConfigError::Validation(errs) => {
                writeln!(f, "Validation failed:")?;
                for e in errs {
                    writeln!(f, "  - {e}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<ConfigError> for BotError {
    fn from(e: ConfigError) -> Self {
        BotError::Validation(e.to_string())
    }
}

/// Parse + validate YAML. The DB is not touched. All discovered errors are returned at once.
pub fn parse(input: &str) -> Result<ValidatedConfig, ConfigError> {
    let raw: GuildConfig =
        serde_yaml::from_str(input).map_err(|e| ConfigError::Yaml(e.to_string()))?;
    validate(raw)
}

fn validate(raw: GuildConfig) -> Result<ValidatedConfig, ConfigError> {
    let mut errs: Vec<String> = Vec::new();

    if raw.version != SUPPORTED_VERSION {
        errs.push(format!(
            "version: {} は未対応（対応バージョン: {SUPPORTED_VERSION}）",
            raw.version
        ));
    }

    let language = raw.guild.and_then(|g| g.language).and_then(|s| {
        let trimmed = s.trim().to_string();
        if trimmed != "ja" && trimmed != "en" {
            errs.push(format!("guild.language は ja/en のみ対応: \"{trimmed}\""));
            None
        } else {
            Some(trimmed)
        }
    });

    let channels = validate_channels(raw.channels.as_ref(), &mut errs);

    // ---- question_presets ----
    let mut preset_names: HashSet<String> = HashSet::new();
    let mut presets: Vec<ValidatedPreset> = Vec::new();
    let mut preset_questions: HashMap<String, Vec<String>> = HashMap::new();
    let mut preset_dropdown_options: HashMap<String, Option<HashSet<String>>> = HashMap::new();
    let mut preset_routing_key_text: HashMap<String, Option<String>> = HashMap::new();

    for (i, p) in raw.question_presets.iter().enumerate() {
        let name = p.name.trim().to_string();
        if name.is_empty() {
            errs.push(format!("question_presets[{i}].name が空"));
            continue;
        }
        if !preset_names.insert(name.clone()) {
            errs.push(format!("question_presets: 名前 \"{name}\" が重複"));
            continue;
        }

        if p.questions.len() > MAX_QUESTIONS_PER_PRESET {
            errs.push(format!(
                "question_presets[\"{name}\"].questions は最大 {MAX_QUESTIONS_PER_PRESET} 個",
            ));
        }

        let mut questions = vec![None; MAX_QUESTIONS_PER_PRESET];
        let mut answers = vec![None; MAX_QUESTIONS_PER_PRESET];
        let mut routing_index: Option<i16> = None;
        let mut question_texts: Vec<String> = Vec::new();
        let mut routing_dropdown_opts: Option<HashSet<String>> = None;
        let mut routing_key_text: Option<String> = None;

        for (qi, q) in p
            .questions
            .iter()
            .enumerate()
            .take(MAX_QUESTIONS_PER_PRESET)
        {
            let text = q.text.trim().to_string();
            if text.is_empty() {
                errs.push(format!(
                    "question_presets[\"{name}\"].questions[{qi}].text が空",
                ));
                continue;
            }
            questions[qi] = Some(text.clone());
            question_texts.push(text.clone());
            if let Some(opts) = &q.answers {
                let cleaned: Vec<String> = opts
                    .iter()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !cleaned.is_empty() {
                    answers[qi] = Some(join_answer_options(&cleaned));
                }
                if q.routing_key && !cleaned.is_empty() {
                    routing_dropdown_opts = Some(cleaned.into_iter().collect());
                }
            }
            if q.routing_key {
                if routing_index.is_some() {
                    errs.push(format!(
                        "question_presets[\"{name}\"]: routing_key は最大 1 つまで",
                    ));
                } else {
                    routing_index = Some(qi as i16);
                    routing_key_text = Some(text);
                }
            }
        }

        preset_questions.insert(name.clone(), question_texts);
        preset_dropdown_options.insert(name.clone(), routing_dropdown_opts);
        preset_routing_key_text.insert(name.clone(), routing_key_text);
        presets.push(ValidatedPreset {
            name,
            questions,
            answers,
            routing_key_index: routing_index,
        });
    }

    // ---- room_groups ----
    let mut group_names: HashSet<String> = HashSet::new();
    let mut groups: Vec<ValidatedGroup> = Vec::new();
    for (i, g) in raw.room_groups.iter().enumerate() {
        let name = g.name.trim().to_string();
        if name.is_empty() {
            errs.push(format!("room_groups[{i}].name が空"));
            continue;
        }
        if !group_names.insert(name.clone()) {
            errs.push(format!("room_groups: 名前 \"{name}\" が重複"));
            continue;
        }
        match parse_channel_id(&g.channel_id) {
            Ok(id) => groups.push(ValidatedGroup {
                name,
                channel_id: id,
            }),
            Err(e) => errs.push(format!("room_groups[\"{name}\"].channel_id: {e}")),
        }
    }

    // ---- rooms ----
    let mut room_vcs: HashSet<i64> = HashSet::new();
    let mut rooms_v: Vec<ValidatedRoom> = Vec::new();
    for (i, r) in raw.rooms.iter().enumerate() {
        let vc = match parse_channel_id(&r.voice_channel_id) {
            Ok(id) => id,
            Err(e) => {
                errs.push(format!("rooms[{i}].voice_channel_id: {e}"));
                continue;
            }
        };
        if !room_vcs.insert(vc) {
            errs.push(format!("rooms: voice_channel_id \"{vc}\" が重複"));
            continue;
        }
        let tc = match &r.text_channel_id {
            Some(s) => match parse_channel_id(s) {
                Ok(id) => Some(id),
                Err(e) => {
                    errs.push(format!("rooms[{i}].text_channel_id: {e}"));
                    None
                }
            },
            None => None,
        };
        if let Some(ref gname) = r.group
            && !group_names.contains(gname)
        {
            errs.push(format!(
                "rooms[{i}].group \"{gname}\" は room_groups に存在しない",
            ));
        }
        if let Some(ref pname) = r.question_preset
            && !preset_names.contains(pname)
        {
            errs.push(format!(
                "rooms[{i}].question_preset \"{pname}\" は question_presets に存在しない",
            ));
        }
        rooms_v.push(ValidatedRoom {
            voice_channel_id: vc,
            text_channel_id: tc,
            group_name: r.group.clone(),
            preset_name: r.question_preset.clone(),
        });
    }

    // ---- routing_rules ----
    let mut routing: Vec<ValidatedRouting> = Vec::new();
    let mut seen_pairs: HashSet<(String, String)> = HashSet::new();
    for (i, pr) in raw.routing_rules.iter().enumerate() {
        let pname = pr.preset.trim().to_string();
        if !preset_names.contains(&pname) {
            errs.push(format!(
                "routing_rules[{i}].preset \"{pname}\" は question_presets に存在しない",
            ));
            continue;
        }
        if preset_routing_key_text
            .get(&pname)
            .is_none_or(|v| v.is_none())
        {
            errs.push(format!(
                "routing_rules[\"{pname}\"]: 対応プリセットに routing_key 質問が無い",
            ));
            // continue to allow other validation errors to surface
        }
        let dropdown_opts = preset_dropdown_options.get(&pname).and_then(|x| x.as_ref());
        for (ri, rule) in pr.rules.iter().enumerate() {
            let when = rule.when.trim().to_string();
            if when.is_empty() {
                errs.push(format!("routing_rules[\"{pname}\"].rules[{ri}].when が空",));
                continue;
            }
            if let Some(opts) = dropdown_opts
                && !opts.contains(&when)
            {
                errs.push(format!(
                    "routing_rules[\"{pname}\"].rules[{ri}].when \"{when}\" は routing_key 質問の選択肢にない",
                ));
            }
            if !seen_pairs.insert((pname.clone(), when.clone())) {
                errs.push(format!(
                    "routing_rules[\"{pname}\"]: when \"{when}\" が重複",
                ));
                continue;
            }
            let channel_id = match parse_channel_id(&rule.channel) {
                Ok(id) => id,
                Err(e) => {
                    errs.push(format!(
                        "routing_rules[\"{pname}\"].rules[{ri}].channel: {e}",
                    ));
                    continue;
                }
            };
            if let Some(tpl) = &rule.template {
                let qs = preset_questions.get(&pname).cloned().unwrap_or_default();
                if let Err(e) = validate_template(tpl, &qs) {
                    errs.push(format!(
                        "routing_rules[\"{pname}\"].rules[{ri}].template: {e}",
                    ));
                    continue;
                }
            }
            routing.push(ValidatedRouting {
                preset_name: pname.clone(),
                match_value: when,
                channel_id,
                template: rule.template.clone(),
            });
        }
    }

    // フォールバック template も検証する。プリセット固有でないので question schema は空。
    if let Some(channels_section) = raw.channels.as_ref()
        && let Some(fb) = channels_section.rental_post_fallback.as_ref()
        && let Some(tpl) = fb.template()
        && let Err(e) = validate_template(tpl, &[])
    {
        errs.push(format!("channels.rental_post_fallback.template: {e}"));
    }

    if !errs.is_empty() {
        return Err(ConfigError::Validation(errs));
    }

    Ok(ValidatedConfig {
        language,
        channels,
        presets,
        groups,
        rooms: rooms_v,
        routing,
    })
}

fn validate_channels(
    section: Option<&ChannelsSection>,
    errs: &mut Vec<String>,
) -> ValidatedChannels {
    let mut out = ValidatedChannels::default();
    let Some(c) = section else {
        return out;
    };
    if let Some(s) = &c.report {
        match parse_channel_id(s) {
            Ok(id) => out.report = Some(id),
            Err(e) => errs.push(format!("channels.report: {e}")),
        }
    }
    if let Some(s) = &c.rental_button {
        match parse_channel_id(s) {
            Ok(id) => out.rental_button = Some(id),
            Err(e) => errs.push(format!("channels.rental_button: {e}")),
        }
    }
    if let Some(s) = &c.room_list {
        match parse_channel_id(s) {
            Ok(id) => out.room_list = Some(id),
            Err(e) => errs.push(format!("channels.room_list: {e}")),
        }
    }
    if let Some(fb) = &c.rental_post_fallback {
        match parse_channel_id(fb.channel()) {
            Ok(id) => out.rental_post_fallback = Some((id, fb.template().map(|s| s.to_string()))),
            Err(e) => errs.push(format!("channels.rental_post_fallback.channel: {e}")),
        }
    }
    out
}

fn validate_template(tpl: &str, question_texts: &[String]) -> Result<(), String> {
    let parsed = template::parse(tpl).map_err(|e| e.to_string())?;
    let schema = TemplateSchema::new(question_texts);
    template::validate(&parsed, &schema).map_err(|e| e.to_string())?;
    Ok(())
}

fn parse_channel_id(s: &str) -> Result<i64, String> {
    let trimmed = s.trim();
    if !(17..=20).contains(&trimmed.len()) {
        return Err(format!(
            "\"{trimmed}\" はチャンネル ID として桁数が不正 (17-20 桁)",
        ));
    }
    if !trimmed.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("\"{trimmed}\" は数値のみ可"));
    }
    trimmed
        .parse::<u64>()
        .map(|v| v as i64)
        .map_err(|e| e.to_string())
}

fn join_answer_options(opts: &[String]) -> String {
    // `,` is the literal separator in DB, `,,` is the escaped comma.
    opts.iter()
        .map(|s| s.replace(',', ",,"))
        .collect::<Vec<_>>()
        .join(",")
}

// ============================================================================
// Apply (DB writes)
// ============================================================================

/// Snapshot of rooms whose deletion would orphan an in-memory rental state.
/// Captured BEFORE the apply tx so the caller can clean up after the commit.
#[derive(Debug, Clone)]
pub struct AffectedRoom {
    pub room_id: i32,
    pub voice_channel_id: Option<i64>,
}

pub async fn find_rooms_to_delete<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    config: &ValidatedConfig,
) -> BotResult<Vec<AffectedRoom>> {
    let kept: HashSet<i64> = config.rooms.iter().map(|r| r.voice_channel_id).collect();
    let current = rooms::Entity::find()
        .filter(rooms::Column::GuildId.eq(guild_id as i64))
        .all(db)
        .await?;
    Ok(current
        .into_iter()
        .filter(|r| match r.voice_channel_id {
            Some(vc) => !kept.contains(&vc),
            None => true,
        })
        .map(|r| AffectedRoom {
            room_id: r.id,
            voice_channel_id: r.voice_channel_id,
        })
        .collect())
}

/// Apply a validated config to the DB. Caller MUST run this inside `with_guild_context`
/// so RLS is enforced. Performs the full DELETE → INSERT replacement atomically.
pub async fn apply<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    config: &ValidatedConfig,
) -> BotResult<()> {
    // 0) Ensure the guilds row exists (required for FKs).
    crate::facade::guild_settings::ensure_guild(db, guild_id).await?;

    // 1) Update guild language if specified.
    if let Some(lang) = &config.language {
        let row = guilds::Entity::find_by_id(guild_id as i64)
            .one(db)
            .await?
            .ok_or_else(|| BotError::NotFound(format!("guild {guild_id}")))?;
        let mut model: guilds::ActiveModel = row.into();
        model.language = Set(lang.clone());
        model.updated_at = Set(chrono::Utc::now().fixed_offset());
        model.update(db).await?;
    }

    // 2) Delete in FK-dependency order. rental_sessions / scheduled_tasks
    //    cascade away when rooms are dropped.
    rental_routing_rules::Entity::delete_many()
        .filter(rental_routing_rules::Column::GuildId.eq(guild_id as i64))
        .exec(db)
        .await?;
    rooms::Entity::delete_many()
        .filter(rooms::Column::GuildId.eq(guild_id as i64))
        .exec(db)
        .await?;
    room_groups::Entity::delete_many()
        .filter(room_groups::Column::GuildId.eq(guild_id as i64))
        .exec(db)
        .await?;
    rental_question_presets::Entity::delete_many()
        .filter(rental_question_presets::Column::GuildId.eq(guild_id as i64))
        .exec(db)
        .await?;
    guild_channels::Entity::delete_many()
        .filter(guild_channels::Column::GuildId.eq(guild_id as i64))
        .exec(db)
        .await?;

    let now = chrono::Utc::now().fixed_offset();

    // 3) Re-insert channels.
    if let Some(id) = config.channels.report {
        insert_channel(db, guild_id, id, CHANNEL_TYPE_REPORT, None, now).await?;
    }
    if let Some(id) = config.channels.rental_button {
        insert_channel(db, guild_id, id, CHANNEL_TYPE_RENTAL_BUTTON, None, now).await?;
    }
    if let Some(id) = config.channels.room_list {
        insert_channel(db, guild_id, id, CHANNEL_TYPE_ROOM_LIST, None, now).await?;
    }
    if let Some((id, tpl)) = &config.channels.rental_post_fallback {
        insert_channel(
            db,
            guild_id,
            *id,
            CHANNEL_TYPE_RENTAL_POST_FALLBACK,
            tpl.clone(),
            now,
        )
        .await?;
    }

    // 4) Re-insert presets. Capture name → new id mapping.
    let mut preset_name_to_id: HashMap<String, i32> = HashMap::new();
    for p in &config.presets {
        let inserted = rental_question_presets::ActiveModel {
            guild_id: Set(guild_id as i64),
            name: Set(p.name.clone()),
            question_1: Set(p.questions[0].clone()),
            question_2: Set(p.questions[1].clone()),
            question_3: Set(p.questions[2].clone()),
            question_4: Set(p.questions[3].clone()),
            question_5: Set(p.questions[4].clone()),
            question_6: Set(p.questions[5].clone()),
            question_7: Set(p.questions[6].clone()),
            question_8: Set(p.questions[7].clone()),
            question_9: Set(p.questions[8].clone()),
            question_10: Set(p.questions[9].clone()),
            answer_1: Set(p.answers[0].clone()),
            answer_2: Set(p.answers[1].clone()),
            answer_3: Set(p.answers[2].clone()),
            answer_4: Set(p.answers[3].clone()),
            answer_5: Set(p.answers[4].clone()),
            answer_6: Set(p.answers[5].clone()),
            answer_7: Set(p.answers[6].clone()),
            answer_8: Set(p.answers[7].clone()),
            answer_9: Set(p.answers[8].clone()),
            answer_10: Set(p.answers[9].clone()),
            routing_key_index: Set(p.routing_key_index),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
        preset_name_to_id.insert(p.name.clone(), inserted.id);
    }

    // 5) Re-insert groups.
    let mut group_name_to_id: HashMap<String, i32> = HashMap::new();
    for g in &config.groups {
        let inserted = room_groups::ActiveModel {
            guild_id: Set(guild_id as i64),
            name: Set(g.name.clone()),
            channel_id: Set(g.channel_id),
            message_id: Set(None),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
        group_name_to_id.insert(g.name.clone(), inserted.id);
    }

    // 6) Re-insert rooms.
    for r in &config.rooms {
        let group_id = r
            .group_name
            .as_ref()
            .and_then(|n| group_name_to_id.get(n).copied());
        let preset_id = r
            .preset_name
            .as_ref()
            .and_then(|n| preset_name_to_id.get(n).copied());
        rooms::ActiveModel {
            guild_id: Set(guild_id as i64),
            voice_channel_id: Set(Some(r.voice_channel_id)),
            text_channel_id: Set(r.text_channel_id),
            is_available: Set(true),
            question_preset_id: Set(preset_id),
            group_id: Set(group_id),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }

    // 7) Re-insert routing rules.
    for rule in &config.routing {
        let preset_id = preset_name_to_id
            .get(&rule.preset_name)
            .copied()
            .ok_or_else(|| {
                BotError::Validation(format!(
                    "routing_rules で参照しているプリセット \"{}\" が再構築後の DB に見当たらない",
                    rule.preset_name,
                ))
            })?;
        rental_routing_rules::ActiveModel {
            guild_id: Set(guild_id as i64),
            preset_id: Set(preset_id),
            match_value: Set(rule.match_value.clone()),
            channel_id: Set(rule.channel_id),
            template: Set(rule.template.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }

    Ok(())
}

async fn insert_channel<C: ConnectionTrait>(
    db: &C,
    guild_id: u64,
    channel_id: i64,
    channel_type: i16,
    template_str: Option<String>,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> BotResult<()> {
    guild_channels::ActiveModel {
        guild_id: Set(guild_id as i64),
        channel_id: Set(channel_id),
        channel_type: Set(channel_type),
        message_id: Set(None),
        template: Set(template_str),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

// ============================================================================
// Dump (DB → YAML)
// ============================================================================

pub async fn dump<C: ConnectionTrait>(db: &C, guild_id: u64) -> BotResult<String> {
    let lang = crate::facade::guild_settings::get_language(db, guild_id).await?;

    let chans = guild_channels::Entity::find()
        .filter(guild_channels::Column::GuildId.eq(guild_id as i64))
        .all(db)
        .await?;

    let mut channels = ChannelsSection {
        report: None,
        rental_button: None,
        room_list: None,
        rental_post_fallback: None,
    };
    for c in chans {
        match c.channel_type {
            CHANNEL_TYPE_REPORT => channels.report = Some(c.channel_id.to_string()),
            CHANNEL_TYPE_RENTAL_BUTTON => channels.rental_button = Some(c.channel_id.to_string()),
            CHANNEL_TYPE_ROOM_LIST => channels.room_list = Some(c.channel_id.to_string()),
            CHANNEL_TYPE_RENTAL_POST_FALLBACK => {
                channels.rental_post_fallback = Some(match c.template {
                    Some(tpl) => FallbackChannel::Object {
                        channel: c.channel_id.to_string(),
                        template: Some(tpl),
                    },
                    None => FallbackChannel::Id(c.channel_id.to_string()),
                });
            }
            _ => {}
        }
    }

    let presets = rental_question_presets::Entity::find()
        .filter(rental_question_presets::Column::GuildId.eq(guild_id as i64))
        .order_by_asc(rental_question_presets::Column::Id)
        .all(db)
        .await?;

    let mut preset_id_to_name: HashMap<i32, String> = HashMap::new();
    let mut preset_sections: Vec<PresetSection> = Vec::new();
    for p in &presets {
        preset_id_to_name.insert(p.id, p.name.clone());
        let qs = [
            (&p.question_1, &p.answer_1),
            (&p.question_2, &p.answer_2),
            (&p.question_3, &p.answer_3),
            (&p.question_4, &p.answer_4),
            (&p.question_5, &p.answer_5),
            (&p.question_6, &p.answer_6),
            (&p.question_7, &p.answer_7),
            (&p.question_8, &p.answer_8),
            (&p.question_9, &p.answer_9),
            (&p.question_10, &p.answer_10),
        ];
        let mut questions: Vec<QuestionSection> = Vec::new();
        for (i, (qopt, aopt)) in qs.iter().enumerate() {
            let Some(text) = qopt.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) else {
                continue;
            };
            let answers = aopt
                .as_ref()
                .map(|raw| qp_facade::parse_answer_options(raw));
            let answers = answers.filter(|v| !v.is_empty());
            let routing_key = p.routing_key_index.map(|n| n as usize) == Some(i);
            questions.push(QuestionSection {
                text: text.to_string(),
                answers,
                routing_key,
            });
        }
        preset_sections.push(PresetSection {
            name: p.name.clone(),
            questions,
        });
    }

    let groups = room_groups::Entity::find()
        .filter(room_groups::Column::GuildId.eq(guild_id as i64))
        .order_by_asc(room_groups::Column::Id)
        .all(db)
        .await?;
    let mut group_id_to_name: HashMap<i32, String> = HashMap::new();
    let mut group_sections: Vec<RoomGroupSection> = Vec::new();
    for g in groups {
        group_id_to_name.insert(g.id, g.name.clone());
        group_sections.push(RoomGroupSection {
            name: g.name,
            channel_id: g.channel_id.to_string(),
        });
    }

    let room_rows = rooms::Entity::find()
        .filter(rooms::Column::GuildId.eq(guild_id as i64))
        .order_by_asc(rooms::Column::Id)
        .all(db)
        .await?;
    let mut room_sections: Vec<RoomSection> = Vec::new();
    for r in room_rows {
        // The schema allows VC-less rooms historically; skip them in YAML output since
        // they can't be addressed by the upload schema either.
        let Some(vc) = r.voice_channel_id else {
            continue;
        };
        room_sections.push(RoomSection {
            voice_channel_id: vc.to_string(),
            text_channel_id: r.text_channel_id.map(|id| id.to_string()),
            group: r.group_id.and_then(|id| group_id_to_name.get(&id).cloned()),
            question_preset: r
                .question_preset_id
                .and_then(|id| preset_id_to_name.get(&id).cloned()),
        });
    }

    let routing_rows = rental_routing_rules::Entity::find()
        .filter(rental_routing_rules::Column::GuildId.eq(guild_id as i64))
        .order_by_asc(rental_routing_rules::Column::PresetId)
        .order_by_asc(rental_routing_rules::Column::Id)
        .all(db)
        .await?;
    let mut routing_by_preset: HashMap<String, Vec<RoutingRuleSection>> = HashMap::new();
    for row in routing_rows {
        let preset_name = match preset_id_to_name.get(&row.preset_id).cloned() {
            Some(n) => n,
            None => continue,
        };
        routing_by_preset
            .entry(preset_name)
            .or_default()
            .push(RoutingRuleSection {
                when: row.match_value,
                channel: row.channel_id.to_string(),
                template: row.template,
            });
    }
    let mut routing_sections: Vec<PresetRoutingSection> = routing_by_preset
        .into_iter()
        .map(|(preset, rules)| PresetRoutingSection { preset, rules })
        .collect();
    routing_sections.sort_by(|a, b| a.preset.cmp(&b.preset));

    let config = GuildConfig {
        version: SUPPORTED_VERSION,
        guild: Some(GuildSection {
            language: Some(lang),
        }),
        channels: Some(channels),
        question_presets: preset_sections,
        room_groups: group_sections,
        rooms: room_sections,
        routing_rules: routing_sections,
    };

    serde_yaml::to_string(&config).map_err(|e| BotError::Validation(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_valid() {
        let yaml = r#"
version: 1
"#;
        let c = parse(yaml).unwrap();
        assert!(c.presets.is_empty());
        assert!(c.rooms.is_empty());
    }

    #[test]
    fn rejects_wrong_version() {
        let yaml = "version: 99\n";
        let err = parse(yaml).unwrap_err();
        match err {
            ConfigError::Validation(es) => assert!(es.iter().any(|e| e.contains("version"))),
            _ => panic!("expected validation error"),
        }
    }

    #[test]
    fn rejects_bad_language() {
        let yaml = "version: 1\nguild:\n  language: fr\n";
        let err = parse(yaml).unwrap_err();
        match err {
            ConfigError::Validation(es) => assert!(es.iter().any(|e| e.contains("language"))),
            _ => panic!("expected validation error"),
        }
    }

    #[test]
    fn rejects_dup_preset_names() {
        let yaml = r#"
version: 1
question_presets:
  - name: foo
    questions: []
  - name: foo
    questions: []
"#;
        let err = parse(yaml).unwrap_err();
        match err {
            ConfigError::Validation(es) => assert!(es.iter().any(|e| e.contains("重複"))),
            _ => panic!(),
        }
    }

    #[test]
    fn rejects_routing_to_unknown_preset() {
        let yaml = r#"
version: 1
routing_rules:
  - preset: ghost
    rules:
      - when: a
        channel: "111111111111111111"
"#;
        let err = parse(yaml).unwrap_err();
        match err {
            ConfigError::Validation(es) => {
                assert!(es.iter().any(|e| e.contains("ghost")));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn rejects_when_not_in_dropdown_options() {
        let yaml = r#"
version: 1
question_presets:
  - name: p
    questions:
      - text: q1
        answers: ["A", "B"]
        routing_key: true
routing_rules:
  - preset: p
    rules:
      - when: Z
        channel: "111111111111111111"
"#;
        let err = parse(yaml).unwrap_err();
        match err {
            ConfigError::Validation(es) => assert!(es.iter().any(|e| e.contains("when"))),
            _ => panic!(),
        }
    }

    #[test]
    fn rejects_unknown_template_var() {
        let yaml = r#"
version: 1
question_presets:
  - name: p
    questions:
      - text: q1
        answers: ["A"]
        routing_key: true
routing_rules:
  - preset: p
    rules:
      - when: A
        channel: "111111111111111111"
        template: "{{wat}}"
"#;
        let err = parse(yaml).unwrap_err();
        match err {
            ConfigError::Validation(es) => {
                assert!(es.iter().any(|e| e.contains("template")));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn accepts_valid_full_config() {
        let yaml = r#"
version: 1
guild:
  language: ja
channels:
  report: "11111111111111111"
  rental_post_fallback:
    channel: "22222222222222222"
    template: "{{user}}"
question_presets:
  - name: 通常
    questions:
      - text: 目的
        answers: ["雑談", "作業"]
        routing_key: true
      - text: 備考
room_groups:
  - name: メイン
    channel_id: "33333333333333333"
rooms:
  - voice_channel_id: "44444444444444444"
    text_channel_id: "55555555555555555"
    group: メイン
    question_preset: 通常
routing_rules:
  - preset: 通常
    rules:
      - when: 雑談
        channel: "66666666666666666"
        template: "{{user}}が雑談を始めました"
"#;
        let c = parse(yaml).unwrap();
        assert_eq!(c.presets.len(), 1);
        assert_eq!(c.presets[0].routing_key_index, Some(0));
        assert_eq!(c.rooms.len(), 1);
        assert_eq!(c.routing.len(), 1);
        assert!(c.channels.rental_post_fallback.is_some());
    }

    #[test]
    fn channel_id_must_be_numeric() {
        let bad = parse(
            r#"
version: 1
channels:
  report: "not-numeric-x-xxxx"
"#,
        );
        assert!(bad.is_err());
    }

    #[test]
    fn dump_then_parse_round_trips_known_yaml() {
        // Render a config from scratch (no DB needed) and run it through serde + validation
        // — the public output (`/download_guild_config`) must always be a valid input.
        let cfg = GuildConfig {
            version: 1,
            guild: Some(GuildSection {
                language: Some("ja".into()),
            }),
            channels: Some(ChannelsSection {
                report: Some("11111111111111111".into()),
                rental_button: None,
                room_list: None,
                rental_post_fallback: Some(FallbackChannel::Object {
                    channel: "22222222222222222".into(),
                    template: Some("{{user}}".into()),
                }),
            }),
            question_presets: vec![PresetSection {
                name: "通常".into(),
                questions: vec![
                    QuestionSection {
                        text: "目的".into(),
                        answers: Some(vec!["雑談".into(), "作業".into()]),
                        routing_key: true,
                    },
                    QuestionSection {
                        text: "備考".into(),
                        answers: None,
                        routing_key: false,
                    },
                ],
            }],
            room_groups: vec![RoomGroupSection {
                name: "メイン".into(),
                channel_id: "33333333333333333".into(),
            }],
            rooms: vec![RoomSection {
                voice_channel_id: "44444444444444444".into(),
                text_channel_id: Some("55555555555555555".into()),
                group: Some("メイン".into()),
                question_preset: Some("通常".into()),
            }],
            routing_rules: vec![PresetRoutingSection {
                preset: "通常".into(),
                rules: vec![RoutingRuleSection {
                    when: "雑談".into(),
                    channel: "66666666666666666".into(),
                    template: Some("{{user}} - {{answer:備考}}".into()),
                }],
            }],
        };
        let yaml = serde_yaml::to_string(&cfg).unwrap();
        let parsed = parse(&yaml).expect("round-trip parse");
        assert_eq!(parsed.presets.len(), 1);
        assert_eq!(parsed.rooms.len(), 1);
        assert_eq!(parsed.routing.len(), 1);
    }

    #[test]
    fn channel_id_length_bounds() {
        // 16 digits → too short
        let bad = parse(
            r#"
version: 1
channels:
  report: "1234567890123456"
"#,
        );
        assert!(bad.is_err());
        // 21 digits → too long
        let bad = parse(
            r#"
version: 1
channels:
  report: "123456789012345678901"
"#,
        );
        assert!(bad.is_err());
    }
}
