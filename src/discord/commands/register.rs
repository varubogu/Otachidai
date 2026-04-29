use crate::error::BotResult;
use twilight_http::Client;
use twilight_model::application::command::CommandType;
use twilight_model::id::{Id, marker::ApplicationMarker};
use twilight_util::builder::command::{ChannelBuilder, CommandBuilder};

pub async fn register_global_commands(
    http: &Client,
    application_id: Id<ApplicationMarker>,
) -> BotResult<()> {
    let commands = vec![
        CommandBuilder::new(
            "register_report_channel",
            "Register the report notification channel",
            CommandType::ChatInput,
        )
        .option(
            ChannelBuilder::new("channel", "The channel to receive timeout notifications")
                .required(true),
        )
        .build(),
        CommandBuilder::new(
            "register_rental_button_channel",
            "Register the channel where the rental button is posted",
            CommandType::ChatInput,
        )
        .option(
            ChannelBuilder::new("channel", "The channel to post the rental button in")
                .required(true),
        )
        .build(),
        CommandBuilder::new(
            "register_room",
            "Register a room (text channel, voice channel, or both)",
            CommandType::ChatInput,
        )
        .option(ChannelBuilder::new("text_channel", "Text channel for this room").required(false))
        .option(ChannelBuilder::new("voice_channel", "Voice channel for this room").required(false))
        .build(),
        CommandBuilder::new(
            "delete_room",
            "Delete a registered room",
            CommandType::ChatInput,
        )
        .option(
            ChannelBuilder::new("text_channel", "Text channel of the room to delete")
                .required(false),
        )
        .option(
            ChannelBuilder::new("voice_channel", "Voice channel of the room to delete")
                .required(false),
        )
        .build(),
        CommandBuilder::new("rent", "Start a rental request", CommandType::ChatInput).build(),
        CommandBuilder::new("help", "Display help information", CommandType::ChatInput).build(),
    ];

    http.interaction(application_id)
        .set_global_commands(&commands)
        .await?;

    tracing::info!("Registered {} global slash commands", commands.len());
    Ok(())
}
