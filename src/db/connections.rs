use crate::config::AppConfig;
use crate::error::{BotError, BotResult};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

pub struct DbPools {
    pub system: DatabaseConnection,
    pub guild: DatabaseConnection,
    pub global: DatabaseConnection,
    pub admin: DatabaseConnection,
}

impl DbPools {
    pub async fn new(config: &AppConfig) -> BotResult<Self> {
        Ok(DbPools {
            system: connect(&config.db_url(&config.system_db)).await?,
            guild: connect(&config.db_url(&config.guild_db)).await?,
            global: connect(&config.db_url(&config.global_db)).await?,
            admin: connect(&config.db_url(&config.admin_db)).await?,
        })
    }
}

async fn connect(url: &str) -> BotResult<DatabaseConnection> {
    let mut opts = ConnectOptions::new(url);
    opts.max_connections(10)
        .min_connections(1)
        .sqlx_logging(false);
    Database::connect(opts).await.map_err(BotError::from)
}
