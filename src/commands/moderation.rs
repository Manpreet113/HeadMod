use poise::serenity_prelude as serenity;
use crate::{Context, Error};

/// Kick a member from the server.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions="KICK_MEMBERS",
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

    let msg = serenity::CreateMessage::new().content(format!(
        "You have been kicked from **{}**.\nReason: {}",
        ctx.guild().unwrap().name,
        reason
    ));

    let _dm_result = member.user.dm(&ctx.http(), msg).await;

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

/// Permanently ban a member, optionally specifying a reason and deleting recent messages.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions="BAN_MEMBERS",
    required_bot_permissions = "BAN_MEMBERS"
)]
pub async fn ban(
    ctx: Context<'_>,
    #[description = "The user to ban"] member: serenity::Member,
    #[description = "Reason for the ban (optional but recommended unless you enjoy chaos)"] reason: Option<String>,
    #[description = "Days of messages to delete (0–7)"] delete: Option<u8>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let reason = reason.as_deref().unwrap_or("No reason provided");
    let dmd = delete.unwrap_or(0);
    // Don't let anyone ban the bot itself
    if member.user.id == ctx.framework().bot_id {
        ctx.say("Nice try. I'm not banning myself.").await?;
        return Ok(());
    }

    if member.user.id == ctx.guild().unwrap().owner_id {
        ctx.say("Owner is notified of your attempt!").await?;
        return Ok(());
    }

    let msg = serenity::CreateMessage::new().content(format!(
        "You have been banned from **{}**.\nReason: {}",
        ctx.guild().unwrap().name,
        reason
    ));

    let _dm_result = member.user.dm(&ctx.http(), msg).await;

    // Attempt the Ban
    match member.ban_with_reason(&ctx.http(), dmd, reason).await {
        Ok(()) => {
            ctx.say(format!(
                "👢 **{}** has been banned.\n**Reason:** {}",
                member.user.name, reason
            ))
                .await?;
        }
        Err(e) => {
            ctx.say(format!(
                "❌ Couldn't ban **{}**: {}",
                member.user.name, e
            ))
                .await?;
        }
    }

    Ok(())
}
