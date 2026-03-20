use poise::serenity_prelude as serenity;
use crate::{Context, Error};
use super::actions::{execute_timeout, ActionResult, TimeoutParams};

/// Temporarily mute a member. Duration format: 10s, 5m, 2h, 7d (max 28d).
#[poise::command(
    slash_command, guild_only,
    default_member_permissions = "MODERATE_MEMBERS",
    required_bot_permissions   = "MODERATE_MEMBERS",
    description_localized("en-US", "Temporarily mute a member (Timeouts). Max 28 days.")
)]
pub async fn timeout(
    ctx: Context<'_>,
    #[description = "Member to timeout"] member: serenity::Member,
    #[description = "Duration (e.g. 10m, 2h, 7d — max 28d)"] duration: String,
    #[description = "Reason"] reason: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;

    if member.user.id == ctx.framework().bot_id {
        ctx.say("Nice try. I'm not timing myself out.").await?;
        return Ok(());
    }

    let reason = reason.as_deref().unwrap_or("No reason provided");

    let result = execute_timeout(TimeoutParams {
        http:     &ctx.http(),
        data:     ctx.data(),
        invoker:  ctx.author(),
        member:   &member,
        reason,
        duration: &duration,
    }).await?;

    match result {
        ActionResult::Ok(embed)         => { ctx.send(poise::CreateReply::default().embed(embed)).await?; }
        ActionResult::DiscordError(e)   => { ctx.say(format!("❌ Couldn't timeout **{}**: {}", member.user.name, e)).await?; }
        ActionResult::InvalidInput(msg) => { ctx.say(format!("❌ {}", msg)).await?; }
    }

    Ok(())
}