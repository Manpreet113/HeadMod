use poise::serenity_prelude as serenity;
use crate::{Context, Error};
use super::actions::{execute_ban, hierarchy_check, ActionResult, BanParams};

/// Permanently ban a member from the server.
#[poise::command(
    slash_command, guild_only,
    default_member_permissions = "BAN_MEMBERS",
    required_bot_permissions   = "BAN_MEMBERS"
)]
pub async fn ban(
    ctx: Context<'_>,
    #[description = "The member to ban"] member: serenity::Member,
    #[description = "Reason for the ban"] reason: Option<String>,
    #[description = "Days of messages to delete (0–7)"] delete: Option<u8>,
) -> Result<(), Error> {
    ctx.defer().await?;

    if member.user.id == ctx.framework().bot_id {
        ctx.say("Nice try. I'm not banning myself.").await?;
        return Ok(());
    }

    let reason      = reason.as_deref().unwrap_or("No reason provided");
    let delete_days = delete.unwrap_or(0).min(7);

    let (invoker_can, bot_can, guild_name) = {
        let guild = ctx.guild().ok_or("Could not fetch guild")?;
        let (ic, bc) = hierarchy_check(&guild, ctx.author().id, ctx.framework().bot_id, member.user.id);
        (ic, bc, guild.name.clone())
    };

    if !invoker_can {
        ctx.say("❌ You can't ban someone at or above your own role.").await?;
        return Ok(());
    }
    if !bot_can {
        ctx.say("❌ I can't ban someone at or above my own role.").await?;
        return Ok(());
    }

    let result = execute_ban(BanParams {
        http: &ctx.http(),
        data: ctx.data(),
        invoker: ctx.author(),
        member: &member,
        guild_name: &guild_name,
        reason,
        delete_days,
    }).await;

    match result {
        ActionResult::Ok { message, .. }    => { ctx.say(message).await?; }
        ActionResult::DiscordError(e)       => { ctx.say(format!("❌ Couldn't ban **{}**: {}", member.user.name, e)).await?; }
        ActionResult::InvalidInput(msg)     => { ctx.say(format!("❌ {}", msg)).await?; }
    }

    Ok(())
}