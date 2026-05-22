use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::error::BotResult;
use crate::facade::group as group_facade;
use crate::i18n::MessageKey;
use fluent_bundle::FluentArgs;
use std::sync::Arc;
use twilight_model::application::interaction::application_command::{
    CommandData, CommandOptionValue,
};
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker, MessageMarker},
};

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

    let deleted = with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        let name = name.clone();
        Box::pin(async move { group_facade::delete_group(txn, guild_id.get(), &name).await })
    })
    .await?;

    let Some(group) = deleted else {
        let mut args = FluentArgs::new();
        args.set("name", name);
        return Ok(state
            .i18n
            .get_with_args(lang, &MessageKey::AdminGroupNotFound, Some(&args)));
    };

    // Best-effort cleanup of the group's status board message.
    if let Some(message_id) = group.message_id {
        let _ = state
            .http
            .delete_message(
                Id::<ChannelMarker>::new(group.channel_id as u64),
                Id::<MessageMarker>::new(message_id as u64),
            )
            .await;
    }

    // Rooms that belonged to this group are now ungrouped; refresh the boards.
    crate::rental::status::trigger(&state, guild_id.get());

    let mut args = FluentArgs::new();
    args.set("name", name);
    Ok(state
        .i18n
        .get_with_args(lang, &MessageKey::AdminGroupDeleted, Some(&args)))
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
