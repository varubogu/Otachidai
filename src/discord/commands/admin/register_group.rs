use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::discord::commands::admin::register_report_channel::extract_channel;
use crate::error::BotResult;
use crate::facade::{group as group_facade, guild_settings};
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
    lang: &str,
) -> BotResult<String> {
    let name = extract_string(data, "name")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let Some(name) = name else {
        return Ok(state.i18n.get(lang, &MessageKey::AdminGroupNameRequired));
    };
    let channel_id = extract_channel(data, "channel")?;

    let existing = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        let name = name.clone();
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id.get()).await?;
            group_facade::find_group_by_name(txn, guild_id.get(), &name).await
        })
    })
    .await?;

    if existing.is_some() {
        let mut args = FluentArgs::new();
        args.set("name", name);
        return Ok(state
            .i18n
            .get_with_args(lang, &MessageKey::AdminGroupExists, Some(&args)));
    }

    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        let name = name.clone();
        Box::pin(async move {
            group_facade::register_group(txn, guild_id.get(), &name, channel_id)
                .await
                .map(|_| ())
        })
    })
    .await?;

    crate::rental::status::trigger(&state, guild_id.get());

    let mut args = FluentArgs::new();
    args.set("name", name);
    args.set("channel", format!("<#{channel_id}>"));
    Ok(state
        .i18n
        .get_with_args(lang, &MessageKey::AdminGroupRegistered, Some(&args)))
}

fn extract_string(data: &CommandData, name: &str) -> Option<String> {
    data.options.iter().find(|o| o.name == name).and_then(|o| {
        if let CommandOptionValue::String(value) = &o.value {
            Some(value.clone())
        } else {
            None
        }
    })
}
