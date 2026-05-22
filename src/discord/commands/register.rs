use crate::error::BotResult;
use twilight_http::Client;
use twilight_model::application::command::CommandType;
use twilight_model::id::{Id, marker::ApplicationMarker};
use twilight_util::builder::command::{ChannelBuilder, CommandBuilder, StringBuilder};

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
            "register_question_preset",
            "Register rental questions preset",
            CommandType::ChatInput,
        )
        .option(StringBuilder::new("name", "Preset name").required(true))
        .option(StringBuilder::new("question_1", "Question 1").required(false))
        .option(
            StringBuilder::new(
                "answer_1",
                "Answer options for Q1 (comma-separated; use ,, for a literal comma)",
            )
            .required(false),
        )
        .option(StringBuilder::new("question_2", "Question 2").required(false))
        .option(
            StringBuilder::new(
                "answer_2",
                "Answer options for Q2 (comma-separated; use ,, for a literal comma)",
            )
            .required(false),
        )
        .option(StringBuilder::new("question_3", "Question 3").required(false))
        .option(
            StringBuilder::new(
                "answer_3",
                "Answer options for Q3 (comma-separated; use ,, for a literal comma)",
            )
            .required(false),
        )
        .option(StringBuilder::new("question_4", "Question 4").required(false))
        .option(
            StringBuilder::new(
                "answer_4",
                "Answer options for Q4 (comma-separated; use ,, for a literal comma)",
            )
            .required(false),
        )
        .option(StringBuilder::new("question_5", "Question 5").required(false))
        .option(
            StringBuilder::new(
                "answer_5",
                "Answer options for Q5 (comma-separated; use ,, for a literal comma)",
            )
            .required(false),
        )
        .option(StringBuilder::new("question_6", "Question 6").required(false))
        .option(
            StringBuilder::new(
                "answer_6",
                "Answer options for Q6 (comma-separated; use ,, for a literal comma)",
            )
            .required(false),
        )
        .option(StringBuilder::new("question_7", "Question 7").required(false))
        .option(
            StringBuilder::new(
                "answer_7",
                "Answer options for Q7 (comma-separated; use ,, for a literal comma)",
            )
            .required(false),
        )
        .option(StringBuilder::new("question_8", "Question 8").required(false))
        .option(
            StringBuilder::new(
                "answer_8",
                "Answer options for Q8 (comma-separated; use ,, for a literal comma)",
            )
            .required(false),
        )
        .option(StringBuilder::new("question_9", "Question 9").required(false))
        .option(
            StringBuilder::new(
                "answer_9",
                "Answer options for Q9 (comma-separated; use ,, for a literal comma)",
            )
            .required(false),
        )
        .option(StringBuilder::new("question_10", "Question 10").required(false))
        .option(
            StringBuilder::new(
                "answer_10",
                "Answer options for Q10 (comma-separated; use ,, for a literal comma)",
            )
            .required(false),
        )
        .build(),
        CommandBuilder::new(
            "register_room",
            "Register a room (text channel, voice channel, or both)",
            CommandType::ChatInput,
        )
        .option(ChannelBuilder::new("text_channel", "Text channel for this room").required(false))
        .option(ChannelBuilder::new("voice_channel", "Voice channel for this room").required(false))
        .option(
            StringBuilder::new("question_preset", "Question preset for this room")
                .autocomplete(true)
                .required(false),
        )
        .option(StringBuilder::new("group", "Group name this room belongs to").required(false))
        .build(),
        CommandBuilder::new(
            "register_group",
            "Register a room group that bundles rooms into one status board",
            CommandType::ChatInput,
        )
        .option(StringBuilder::new("name", "Group name").required(true))
        .option(
            ChannelBuilder::new(
                "channel",
                "The channel to post this group's status board in",
            )
            .required(true),
        )
        .build(),
        CommandBuilder::new(
            "delete_group",
            "Delete a room group",
            CommandType::ChatInput,
        )
        .option(StringBuilder::new("name", "Group name").required(true))
        .build(),
        CommandBuilder::new(
            "set_room_group",
            "Change which group a registered room belongs to",
            CommandType::ChatInput,
        )
        .option(
            ChannelBuilder::new("text_channel", "Text channel of the target room").required(false),
        )
        .option(
            ChannelBuilder::new("voice_channel", "Voice channel of the target room")
                .required(false),
        )
        .option(
            StringBuilder::new("group", "Group name to assign (leave empty to ungroup)")
                .required(false),
        )
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
