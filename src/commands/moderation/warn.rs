use poise::serenity_prelude as serenity;
use crate::{Context, Error};
use super::actions::{execute_warn, ActionResult, WarnParams};

/// Warn a member. At the threshold, an automatic punishment is applied.
#[poise::command(
    slash_command, guild_only,
    default_member_permissions = "MODERATE_MEMBERS",
    required_bot_permissions   = "MODERATE_MEMBERS",
    subcommands("add", "list", "clear"),
    description_localized("en-US", "Manage user warnings and penalty escalations.")
)]
pub async fn warn(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add a formal warning to a member.
#[poise::command(slash_command, guild_only, description_localized("en-US", "Issue a formal warning to a member."))]
pub async fn add(
    ctx: Context<'_>,
    #[description = "Member to warn"] member: serenity::Member,
    #[description = "Reason for the warning"] reason: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let guild_name = ctx.guild_id().unwrap().name(ctx.cache()).unwrap_or_else(|| "the server".to_string());

    let result = execute_warn(WarnParams {
        http:       &ctx.http(),
        data:       ctx.data(),
        invoker:    ctx.author(),
        member:     &member,
        guild_name: &guild_name,
        reason:     &reason,
    }).await?;

    match result {
        ActionResult::Ok(embed)             => { ctx.send(poise::CreateReply::default().embed(embed)).await?; }
        ActionResult::DiscordError(e)       => { ctx.say(format!("❌ Error: {}", e)).await?; }
        ActionResult::InvalidInput(msg)     => { ctx.say(format!("❌ {}", msg)).await?; }
    }

    Ok(())
}

/// List all formal warnings for a member.
#[poise::command(slash_command, guild_only, description_localized("en-US", "View all warnings recorded for a specific member."))]
pub async fn list(
    ctx: Context<'_>,
    #[description = "Member to check"] member: serenity::Member,
) -> Result<(), Error> {
    ctx.defer().await?;

    let gid = member.guild_id.get() as i64;
    let tid = member.user.id.get() as i64;
    
    let warns = sqlx::query!(
        "SELECT reason, moderator_id, created_at FROM cases WHERE guild_id = ? AND target_id = ? AND action_type = 'warn' ORDER BY created_at ASC",
        gid, tid
    ).fetch_all(&ctx.data().db).await?;

    if warns.is_empty() {
        ctx.say(format!("✅ **{}** has no warnings.", member.user.name)).await?;
    } else {
        let lines: Vec<String> = warns.iter().enumerate().map(|(i, w)| format!(
            "**{}**. {} — by <@{}> on <t:{}:D>",
            i + 1, w.reason, w.moderator_id, w.created_at.and_utc().timestamp(),
        )).collect();

        ctx.say(format!(
            "⚠️ **{}** has **{}** warning(s):\n{}",
            member.user.name, warns.len(), lines.join("\n"),
        )).await?;
    }

    Ok(())
}

/// Clear all formal warnings for a member.
#[poise::command(slash_command, guild_only, description_localized("en-US", "Remove all warning records for a member."))]
pub async fn clear(
    ctx: Context<'_>,
    #[description = "Member to clear warnings for"] member: serenity::Member,
) -> Result<(), Error> {
    ctx.defer().await?;
    let gid = member.guild_id.get() as i64;
    let tid = member.user.id.get() as i64;
    
    sqlx::query!(
        "DELETE FROM cases WHERE guild_id = ? AND target_id = ? AND action_type = 'warn'",
        gid, tid
    ).execute(&ctx.data().db).await?;
    
    ctx.say(format!("✅ Cleared all warnings for **{}**.", member.user.name)).await?;
    Ok(())
}