use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::BotResult;
use crate::facade::guild_settings;
use crate::i18n::MessageKey;
use fluent_bundle::FluentArgs;
use std::sync::Arc;
use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};
use twilight_model::id::{Id, marker::GuildMarker};

pub async fn handle(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    data: &CommandData,
) -> BotResult<String> {
    let channel_id = extract_channel(data, "channel")?;

    let lang = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id.get()).await?;
            guild_settings::get_language(txn, guild_id.get()).await
        })
    })
    .await
    .unwrap_or_else(|_| "en".to_string());

    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            guild_settings::set_report_channel(txn, guild_id.get(), channel_id).await
        })
    })
    .await?;

    let mut args = FluentArgs::new();
    args.set("channel", format!("<#{channel_id}>"));
    Ok(state.i18n.get_with_args(
        &lang,
        &MessageKey::AdminReportChannelRegistered,
        Some(&args),
    ))
}

pub fn extract_channel(data: &CommandData, name: &str) -> BotResult<u64> {
    data.options
        .iter()
        .find(|o| o.name == name)
        .and_then(|o| {
            if let CommandOptionValue::Channel(id) = &o.value {
                Some(id.get())
            } else {
                None
            }
        })
        .ok_or_else(|| crate::error::BotError::Validation(format!("missing option: {name}")))
}
