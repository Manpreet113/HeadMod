use poise::serenity_prelude as serenity;
use crate::{Context, Error};

/// View a user's moderation history.
#[poise::command(
    slash_command, guild_only,
    default_member_permissions = "MODERATE_MEMBERS",
    description_localized("en-US", "View the moderation history (warnings, bans, kicks) for a specific user.")
)]
pub async fn history(
    ctx: Context<'_>,
    #[description = "The user to check"] user: serenity::User,
) -> Result<(), Error> {
    ctx.defer().await?;

    let gid = ctx.guild_id().unwrap().get() as i64;
    let tid = user.id.get() as i64;

    let cases = sqlx::query!(
        "SELECT id, action_type, reason, created_at FROM cases WHERE guild_id = ? AND target_id = ? ORDER BY id DESC LIMIT 25",
        gid, tid
    ).fetch_all(&ctx.data().db).await?;

    if cases.is_empty() {
        ctx.say(format!("✅ **{}** has a clean record.", user.name)).await?;
    } else {
        let lines: Vec<String> = cases.into_iter().map(|c| {
            format!("`#{}` **{}** — {} (<t:{}:d>)", c.id.unwrap_or(0), c.action_type.to_uppercase(), c.reason, c.created_at.and_utc().timestamp())
        }).collect();

        let embed = serenity::CreateEmbed::new()
            .title(format!("Moderation History for {}", user.name))
            .description(lines.join("\n"))
            .colour(serenity::Colour::ORANGE);

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    }

    Ok(())
}
