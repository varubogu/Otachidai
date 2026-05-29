use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::{BotError, BotResult};
use crate::facade::guild_config::{self, ConfigError};
use crate::i18n::MessageKey;
use crate::rental::routing;
use fluent_bundle::FluentArgs;
use std::sync::Arc;
use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};
use twilight_model::id::{Id, marker::GuildMarker};

const MAX_YAML_BYTES: u64 = 256 * 1024;

pub async fn handle(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    interaction_data: &CommandData,
    lang: &str,
) -> BotResult<String> {
    // 1) Extract attachment id from options, look it up in resolved.attachments.
    let attachment_id = interaction_data
        .options
        .iter()
        .find(|o| o.name == "file")
        .and_then(|o| {
            if let CommandOptionValue::Attachment(id) = o.value {
                Some(id)
            } else {
                None
            }
        })
        .ok_or_else(|| BotError::Validation("missing option: file".to_string()))?;

    let attachment = interaction_data
        .resolved
        .as_ref()
        .and_then(|r| r.attachments.get(&attachment_id))
        .ok_or_else(|| {
            BotError::Validation(
                state
                    .i18n
                    .get(lang, &MessageKey::BotConfigUploadErrorAttachment),
            )
        })?;

    if attachment.size > MAX_YAML_BYTES {
        return Ok(state
            .i18n
            .get(lang, &MessageKey::BotConfigUploadErrorAttachment));
    }

    // 2) Fetch attachment body. Discord CDN serves on a publicly-reachable URL.
    let bytes = match reqwest::get(&attachment.url).await {
        Ok(resp) => match resp.bytes().await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("attachment fetch failed: {e}");
                return Ok(state
                    .i18n
                    .get(lang, &MessageKey::BotConfigUploadErrorAttachment));
            }
        },
        Err(e) => {
            tracing::warn!("attachment fetch failed: {e}");
            return Ok(state
                .i18n
                .get(lang, &MessageKey::BotConfigUploadErrorAttachment));
        }
    };

    let yaml = match std::str::from_utf8(&bytes) {
        Ok(s) => s,
        Err(_) => {
            return Ok(state
                .i18n
                .get(lang, &MessageKey::BotConfigUploadErrorAttachment));
        }
    };

    // 3) Parse + validate (no DB writes).
    let config = match guild_config::parse(yaml) {
        Ok(c) => c,
        Err(ConfigError::Yaml(s)) => {
            let mut args = FluentArgs::new();
            args.set("detail", s);
            return Ok(state.i18n.get_with_args(
                lang,
                &MessageKey::BotConfigUploadErrorYaml,
                Some(&args),
            ));
        }
        Err(ConfigError::Validation(es)) => {
            let mut args = FluentArgs::new();
            args.set("detail", es.join("\n"));
            return Ok(state.i18n.get_with_args(
                lang,
                &MessageKey::BotConfigUploadErrorValidation,
                Some(&args),
            ));
        }
    };

    // 4) Apply within a guild-RLS transaction. Discover affected rooms first so we can
    //    notify their hosts after the commit.
    let affected = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        let config = config.clone();
        Box::pin(async move {
            let to_delete =
                guild_config::find_rooms_to_delete(txn, guild_id.get(), &config).await?;
            guild_config::apply(txn, guild_id.get(), &config).await?;
            Ok::<_, BotError>(to_delete)
        })
    })
    .await?;

    let affected_count = affected.len();
    if affected_count > 0 {
        let affected_ids: Vec<i32> = affected.iter().map(|a| a.room_id).collect();
        routing::force_release_for_rooms(&state, guild_id.get(), &affected_ids, lang).await;
    }

    // 5) Refresh status board.
    crate::rental::status::trigger(&state, guild_id.get());

    if affected_count > 0 {
        let mut args = FluentArgs::new();
        args.set("count", affected_count as i64);
        Ok(state.i18n.get_with_args(
            lang,
            &MessageKey::BotConfigUploadActiveSessionsReleased,
            Some(&args),
        ))
    } else {
        Ok(state.i18n.get(lang, &MessageKey::BotConfigUploadSuccess))
    }
}
