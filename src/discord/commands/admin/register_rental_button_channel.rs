use crate::app_state::AppState;
use crate::db::rls::with_guild_context;
use crate::discord::commands::admin::register_report_channel::extract_channel;
use crate::discord::components::rental_button::build_rental_button;
use crate::error::BotResult;
use crate::facade::guild_settings;
use crate::i18n::MessageKey;
use fluent_bundle::FluentArgs;
use std::sync::Arc;
use twilight_model::application::interaction::application_command::CommandData;
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker},
};

pub async fn handle(
    state: Arc<AppState>,
    guild_id: Id<GuildMarker>,
    data: &CommandData,
    lang: &str,
) -> BotResult<String> {
    let channel_id = extract_channel(data, "channel")?;

    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            guild_settings::ensure_guild(txn, guild_id.get()).await?;
            Ok(())
        })
    })
    .await?;

    with_guild_context(&state.db.guild, guild_id.get(), |txn| {
        Box::pin(async move {
            guild_settings::set_rental_button_channel(txn, guild_id.get(), channel_id).await?;
            // Drop any stale status-board message id from a previous registration.
            guild_settings::set_rental_button_message_id(txn, guild_id.get(), None).await
        })
    })
    .await?;

    let button_label = state.i18n.get(lang, &MessageKey::RentButtonLabel);
    let components = build_rental_button(button_label);

    state
        .http
        .create_message(Id::<ChannelMarker>::new(channel_id))
        .components(&components)
        .await?;

    // Post the rental-status board directly below the button. Best-effort: a
    // failure here must not fail the registration itself.
    if let Err(e) = crate::rental::status::refresh(&state, guild_id.get()).await {
        tracing::warn!("Failed to post initial rental status board: {e}");
    }

    let mut args = FluentArgs::new();
    args.set("channel", format!("<#{channel_id}>"));
    Ok(state
        .i18n
        .get_with_args(lang, &MessageKey::AdminRentalButtonRegistered, Some(&args)))
}
