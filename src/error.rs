use thiserror::Error;

#[derive(Debug, Error)]
pub enum BotError {
    #[error("Database error: {0}")]
    Db(#[from] sea_orm::DbErr),

    #[error("Discord HTTP error: {0}")]
    Http(#[from] twilight_http::Error),

    #[error("Discord HTTP deserialize error: {0}")]
    HttpDeserialize(#[from] twilight_http::response::DeserializeBodyError),

    #[error("Environment variable missing: {0}")]
    Env(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Permission denied")]
    PermissionDenied,

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("I18n error: {0}")]
    I18n(String),
}

pub type BotResult<T> = Result<T, BotError>;
