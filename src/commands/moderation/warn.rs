use poise::serenity_prelude as serenity;
use crate::{Context, Error};
use super::actions::{execute_warn, ActionResult, WarnParams};

/// Warn a member. At the threshold, an automatic timeout is applied.
#[poise::command(
    slash_command, guild_only,
    default_member_permissions = "MODERATE_MEMBERS",
    required_bot_permissions   = "MODERATE_MEMBERS",
    subcommands("add", "list", "clear")
)]
pub async fn warn(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add a warning to a member.
#[poise::command(slash_command, guild_only)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "Member to warn"] member: serenity::Member,
    #[description = "Reason for the warning"] reason: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let result = execute_warn(WarnParams {
        http:    &ctx.http(),
        data:    ctx.data(),
        invoker: ctx.author(),
        member:  &member,
        reason:  &reason,
    }).await?;

    match result {
        ActionResult::Ok { message, .. }    => { ctx.say(message).await?; }
        ActionResult::DiscordError(e)       => { ctx.say(format!("❌ Error: {}", e)).await?; }
        ActionResult::InvalidInput(msg)     => { ctx.say(format!("❌ {}", msg)).await?; }
    }

    Ok(())
}

/// List all warnings for a member.
#[poise::command(slash_command, guild_only)]
pub async fn list(
    ctx: Context<'_>,
    #[description = "Member to check"] member: serenity::Member,
) -> Result<(), Error> {
    ctx.defer().await?;

    let data  = ctx.data();
    let warns = data.warns.get(&member.user.id);
    let warns = warns.as_deref();

    if warns.map(|w| w.is_empty()).unwrap_or(true) {
        ctx.say(format!("✅ **{}** has no warnings.", member.user.name)).await?;
    } else {
        let warns = warns.unwrap();
        let lines: Vec<String> = warns.iter().enumerate().map(|(i, w)| format!(
            "**{}**. {} — by <@{}> on <t:{}:D>",
            i + 1, w.reason, w.moderator, w.timestamp.timestamp(),
        )).collect();

        ctx.say(format!(
            "⚠️ **{}** has **{}** warning(s):\n{}",
            member.user.name, warns.len(), lines.join("\n"),
        )).await?;
    }

    Ok(())
}

/// Clear all warnings for a member.
#[poise::command(slash_command, guild_only)]
pub async fn clear(
    ctx: Context<'_>,
    #[description = "Member to clear warnings for"] member: serenity::Member,
) -> Result<(), Error> {
    ctx.defer().await?;
    ctx.data().warns.remove(&member.user.id);
    ctx.say(format!("✅ Cleared all warnings for **{}**.", member.user.name)).await?;
    Ok(())
}