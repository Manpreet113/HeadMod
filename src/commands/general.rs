use crate::{Context, Error};

/// A quick way to check the bot is alive.
#[poise::command(slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("I'm up alright!").await?;
    Ok(())
}
