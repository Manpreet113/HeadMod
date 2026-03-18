use poise::serenity_prelude as serenity;
use crate::{Context, Error};

/// Kick a member from the server.
#[poise::command(
    slash_command,
    guild_only,
    required_permissions = "KICK_MEMBERS",
    required_bot_permissions = "KICK_MEMBERS"
)]
pub async fn kick(
    ctx: Context<'_>,
    #[description = "The member to kick"] member: serenity::Member,
    #[description = "Reason for the kick"] reason: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let reason = reason.as_deref().unwrap_or("No reason provided");
    // Don't let anyone kick the bot itself
    if member.user.id == ctx.framework().bot_id {
        ctx.say("Nice try. I'm not kicking myself.").await?;
        return Ok(());
    }

    if member.user.id == ctx.guild().unwrap().owner_id {
        ctx.say("Quite a rebellion but you can't just kick the server owner!").await?;
        return Ok(());
    }

    // Attempt the kick
    match member.kick_with_reason(&ctx.http(), reason).await {
        Ok(()) => {
            ctx.say(format!(
                "👢 **{}** has been kicked.\n**Reason:** {}",
                member.user.name, reason
            ))
            .await?;
        }
        Err(e) => {
            ctx.say(format!(
                "❌ Couldn't kick **{}**: {}",
                member.user.name, e
            ))
            .await?;
        }
    }

    Ok(())
}


