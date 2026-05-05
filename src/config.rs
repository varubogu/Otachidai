use crate::error::{BotError, BotResult};
use crate::language::normalize_language;

#[derive(Debug, Clone)]
pub struct DbRoleConfig {
    pub user: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub discord_token: String,
    pub bot_admin_server_id: String,
    pub db_host: String,
    pub db_port: u16,
    pub db_name: String,
    pub system_db: DbRoleConfig,
    pub guild_db: DbRoleConfig,
    pub global_db: DbRoleConfig,
    pub admin_db: DbRoleConfig,
    pub rust_log: String,
    pub app_language: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> BotResult<Self> {
        Ok(AppConfig {
            discord_token: require_env("DISCORD_TOKEN")?,
            bot_admin_server_id: require_env("BOT_ADMIN_SERVER_ID")?,
            db_host: std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
            db_port: std::env::var("DB_PORT")
                .unwrap_or_else(|_| "5432".to_string())
                .parse()
                .map_err(|_| BotError::Parse("DB_PORT must be a number".to_string()))?,
            db_name: std::env::var("DB_NAME").unwrap_or_else(|_| "otachidai_bot_db".to_string()),
            system_db: DbRoleConfig {
                user: require_env("SYSTEM_DB_USER")?,
                password: require_env("SYSTEM_DB_PASSWORD")?,
            },
            guild_db: DbRoleConfig {
                user: require_env("GUILD_DB_USER")?,
                password: require_env("GUILD_DB_PASSWORD")?,
            },
            global_db: DbRoleConfig {
                user: require_env("GLOBAL_DB_USER")?,
                password: require_env("GLOBAL_DB_PASSWORD")?,
            },
            admin_db: DbRoleConfig {
                user: require_env("ADMIN_DB_USER")?,
                password: require_env("ADMIN_DB_PASSWORD")?,
            },
            rust_log: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            app_language: optional_language_env("APP_LANGUAGE")?,
        })
    }

    pub fn db_url(&self, role: &DbRoleConfig) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            role.user, role.password, self.db_host, self.db_port, self.db_name
        )
    }
}

fn require_env(key: &str) -> BotResult<String> {
    std::env::var(key).map_err(|_| BotError::Env(key.to_string()))
}

fn optional_language_env(key: &str) -> BotResult<Option<String>> {
    let Ok(value) = std::env::var(key) else {
        return Ok(None);
    };
    normalize_language(&value)
        .map(|lang| Some(lang.to_string()))
        .ok_or_else(|| BotError::Parse(format!("{key} must be ja or en")))
}
