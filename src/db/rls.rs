use crate::error::{BotError, BotResult};
use futures::future::BoxFuture;
use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction, TransactionTrait};

pub async fn with_guild_context<F, T>(db: &DatabaseConnection, guild_id: u64, f: F) -> BotResult<T>
where
    F: for<'c> FnOnce(&'c DatabaseTransaction) -> BoxFuture<'c, BotResult<T>>,
{
    let txn = db.begin().await?;
    txn.execute_unprepared(&format!("SET LOCAL app.current_guild_id = '{guild_id}'"))
        .await
        .map_err(BotError::from)?;
    let result = f(&txn).await?;
    txn.commit().await?;
    Ok(result)
}
