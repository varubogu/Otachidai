use chrono::{Duration, Utc};
use sea_orm::{ColumnTrait, Database, DatabaseConnection, EntityTrait, QueryFilter};

#[derive(Debug)]
struct CleanupConfig {
    db_url: String,
    retention_days: i64,
    execution_mode: ExecutionMode,
    schedule_hour_utc: u32,
    schedule_minute_utc: u32,
    run_on_startup: bool,
}

#[derive(Debug)]
enum ExecutionMode {
    Once,
    Scheduler,
}

impl CleanupConfig {
    fn from_env() -> anyhow::Result<Self> {
        let db_host = std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string());
        let db_port = std::env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string());
        let db_user = std::env::var("DB_USER")?;
        let db_password = std::env::var("DB_PASSWORD")?;
        let db_name = std::env::var("DB_NAME").unwrap_or_else(|_| "otachidai_bot_db".to_string());

        let db_url = format!("postgres://{db_user}:{db_password}@{db_host}:{db_port}/{db_name}");

        let execution_mode = match std::env::var("CLEANUP_EXECUTION_MODE")
            .as_deref()
            .unwrap_or("once")
        {
            "scheduler" => ExecutionMode::Scheduler,
            _ => ExecutionMode::Once,
        };

        Ok(CleanupConfig {
            db_url,
            retention_days: std::env::var("CLEANUP_RETENTION_DAYS")
                .unwrap_or_else(|_| "7".to_string())
                .parse()
                .unwrap_or(7),
            execution_mode,
            schedule_hour_utc: std::env::var("CLEANUP_SCHEDULE_HOUR_UTC")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .unwrap_or(3),
            schedule_minute_utc: std::env::var("CLEANUP_SCHEDULE_MINUTE_UTC")
                .unwrap_or_else(|_| "0".to_string())
                .parse()
                .unwrap_or(0),
            run_on_startup: std::env::var("CLEANUP_RUN_ON_STARTUP")
                .map(|v| v == "true")
                .unwrap_or(false),
        })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::from_filename(".env.maintenance").ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".parse().unwrap()),
        )
        .init();

    let config = CleanupConfig::from_env()?;
    let db = Database::connect(&config.db_url).await?;

    match config.execution_mode {
        ExecutionMode::Once => {
            tracing::info!("Running cleanup (once mode)");
            run_cleanup(&db, config.retention_days).await?;
        }
        ExecutionMode::Scheduler => {
            if config.run_on_startup {
                tracing::info!("Running cleanup on startup");
                run_cleanup(&db, config.retention_days).await?;
            }
            loop {
                let delay =
                    secs_until_next_run(config.schedule_hour_utc, config.schedule_minute_utc);
                tracing::info!("Next cleanup in {}s", delay.as_secs());
                tokio::time::sleep(delay).await;
                tracing::info!("Running scheduled cleanup");
                if let Err(e) = run_cleanup(&db, config.retention_days).await {
                    tracing::error!("Cleanup failed: {e}");
                }
            }
        }
    }

    Ok(())
}

async fn run_cleanup(db: &DatabaseConnection, retention_days: i64) -> anyhow::Result<()> {
    use otachidai::entities::{notifications, rental_sessions, scheduled_tasks};

    let cutoff = (Utc::now() - Duration::days(retention_days)).fixed_offset();

    // Delete sent notifications older than retention period
    let deleted = notifications::Entity::delete_many()
        .filter(notifications::Column::Sent.eq(true))
        .filter(notifications::Column::CreatedAt.lt(cutoff))
        .exec(db)
        .await?;
    tracing::info!("Deleted {} old notifications", deleted.rows_affected);

    // Delete processed scheduled tasks older than retention period
    let deleted = scheduled_tasks::Entity::delete_many()
        .filter(scheduled_tasks::Column::Processed.eq(true))
        .filter(scheduled_tasks::Column::CreatedAt.lt(cutoff))
        .exec(db)
        .await?;
    tracing::info!("Deleted {} old scheduled tasks", deleted.rows_affected);

    // Delete released rental sessions older than retention period
    let deleted = rental_sessions::Entity::delete_many()
        .filter(
            rental_sessions::Column::State.eq(otachidai::entities::rental_sessions::STATE_RELEASED),
        )
        .filter(rental_sessions::Column::EndedAt.lt(cutoff))
        .exec(db)
        .await?;
    tracing::info!("Deleted {} old rental sessions", deleted.rows_affected);

    Ok(())
}

fn secs_until_next_run(hour: u32, minute: u32) -> std::time::Duration {
    let now = Utc::now();
    let mut next = now
        .date_naive()
        .and_hms_opt(hour, minute, 0)
        .map(|dt| dt.and_utc())
        .unwrap_or(now);

    if next <= now {
        next += Duration::days(1);
    }

    let secs = (next - now).num_seconds().max(0) as u64;
    std::time::Duration::from_secs(secs)
}
