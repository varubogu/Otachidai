use crate::error::BotResult;
use twilight_http::Client;
use twilight_model::application::command::CommandType;
use twilight_model::id::{Id, marker::ApplicationMarker};
use twilight_util::builder::command::{AttachmentBuilder, CommandBuilder};

pub async fn register_global_commands(
    http: &Client,
    application_id: Id<ApplicationMarker>,
) -> BotResult<()> {
    let commands = vec![
        CommandBuilder::new(
            "upload_guild_config",
            "Upload the whole-guild YAML configuration",
            CommandType::ChatInput,
        )
        .option(
            AttachmentBuilder::new("file", "The YAML file containing the guild configuration")
                .required(true),
        )
        .build(),
        CommandBuilder::new(
            "download_guild_config",
            "Download the current whole-guild configuration as a YAML file",
            CommandType::ChatInput,
        )
        .build(),
        CommandBuilder::new(
            "list_question_presets",
            "List registered question presets",
            CommandType::ChatInput,
        )
        .build(),
        CommandBuilder::new(
            "list_rooms",
            "List registered rooms",
            CommandType::ChatInput,
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
