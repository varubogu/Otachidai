use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::facade::guild_settings;
use twilight_model::id::{Id, marker::GuildMarker};

pub fn normalize_language(language: &str) -> Option<&'static str> {
    let language = language.trim().to_ascii_lowercase().replace('_', "-");
    match language.split('-').next() {
        Some("ja") => Some("ja"),
        Some("en") => Some("en"),
        _ => None,
    }
}

pub async fn resolve_language(
    state: &AppState,
    guild_id: Id<GuildMarker>,
    discord_locale: Option<&str>,
) -> String {
    if let Some(lang) = discord_locale.and_then(normalize_language) {
        return lang.to_string();
    }

    if let Some(lang) = state
        .config_language
        .as_deref()
        .and_then(normalize_language)
    {
        return lang.to_string();
    }

    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move { guild_settings::get_language(txn, guild_id.get()).await })
    })
    .await
    .unwrap_or_else(|_| guild_settings::DEFAULT_LANGUAGE.to_string())
}

#[cfg(test)]
mod tests {
    use super::normalize_language;

    #[test]
    fn normalize_supported_discord_locales() {
        assert_eq!(normalize_language("ja"), Some("ja"));
        assert_eq!(normalize_language("ja-JP"), Some("ja"));
        assert_eq!(normalize_language("en-US"), Some("en"));
        assert_eq!(normalize_language("en_GB"), Some("en"));
    }

    #[test]
    fn normalize_ignores_unsupported_locales() {
        assert_eq!(normalize_language("ko"), None);
        assert_eq!(normalize_language(""), None);
    }
}
