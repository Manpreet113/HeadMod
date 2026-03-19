use poise::serenity_prelude as serenity;
use crate::{Context, Error};
use super::actions::{execute_kick, hierarchy_check, ActionResult, KickParams};

/// Kick a member from the server.
#[poise::command(
    slash_command, guild_only,
    default_member_permissions = "KICK_MEMBERS",
    required_bot_permissions   = "KICK_MEMBERS"
)]
pub async fn kick(
    ctx: Context<'_>,
    #[description = "The member to kick"] member: serenity::Member,
    #[description = "Reason for the kick"] reason: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;

    if member.user.id == ctx.framework().bot_id {
        ctx.say("Nice try. I'm not kicking myself.").await?;
        return Ok(());
    }

    let reason = reason.as_deref().unwrap_or("No reason provided");

    // Extract everything we need from the CacheRef in a sync block.
    let (invoker_can, bot_can, guild_name) = {
        let guild = ctx.guild().ok_or("Could not fetch guild")?;
        let (ic, bc) = hierarchy_check(&guild, ctx.author().id, ctx.framework().bot_id, member.user.id);
        (ic, bc, guild.name.clone())
    };

    if !invoker_can {
        ctx.say("❌ You can't kick someone at or above your own role.").await?;
        return Ok(());
    }
    if !bot_can {
        ctx.say("❌ I can't kick someone at or above my own role.").await?;
        return Ok(());
    }

    let result = execute_kick(KickParams {
        http: &ctx.http(),
        data: ctx.data(),
        invoker: ctx.author(),
        member: &member,
        guild_name: &guild_name,
        reason,
    }).await;

    match result {
        ActionResult::Ok { message, .. }     => { ctx.say(message).await?; }
        ActionResult::DiscordError(e)        => { ctx.say(format!("❌ Couldn't kick **{}**: {}", member.user.name, e)).await?; }
        ActionResult::InvalidInput(msg)      => { ctx.say(format!("❌ {}", msg)).await?; }
    }

    Ok(())
}