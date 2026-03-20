use crate::{Context, Error};

/// A quick way to check if the bot is online and responding.
#[poise::command(slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("I'm up alright!").await?;
    Ok(())
}

/// Display help information and usage for all available commands.
#[poise::command(slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Command to get help for"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            extra_text_at_bottom: "Use /help <command> for details on a specific command.",
            ephemeral: true,
            show_subcommands: true,
            include_description: true,
            ..Default::default()
        },
    ).await?;
    Ok(())
}