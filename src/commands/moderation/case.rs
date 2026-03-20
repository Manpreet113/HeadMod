use poise::serenity_prelude as serenity;
use crate::{Context, Error};

/// Manage and view moderation cases.
#[poise::command(
    slash_command, guild_only,
    default_member_permissions = "MODERATE_MEMBERS",
    subcommands("view", "edit", "delete"),
    description_localized("en-US", "Manage and view detailed moderation case history.")
)]
pub async fn case(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// View details for a specific case.
#[poise::command(slash_command, guild_only, description_localized("en-US", "View information about a specific case."))]
pub async fn view(
    ctx: Context<'_>,
    #[description = "The case ID"] id: i64,
) -> Result<(), Error> {
    ctx.defer().await?;

    let gid = ctx.guild_id().unwrap().get() as i64;

    let row = sqlx::query!(
        "SELECT target_id, moderator_id, action_type, reason, created_at, duration_secs FROM cases WHERE id = ? AND guild_id = ?",
        id, gid
    ).fetch_optional(&ctx.data().db).await?;

    match row {
        Some(r) => {
            let mut desc = format!("**Action:** {}\n**Target:** <@{}>\n**Moderator:** <@{}>\n**Reason:** {}\n**Time:** <t:{}:F>",
                r.action_type.to_uppercase(), r.target_id, r.moderator_id, r.reason, r.created_at.and_utc().timestamp());
            if let Some(d) = r.duration_secs {
                desc.push_str(&format!("\n**Duration:** {}s", d));
            }

            let embed = serenity::CreateEmbed::new()
                .title(format!("Case #{}", id))
                .description(desc)
                .colour(serenity::Colour::BLITZ_BLUE);

            ctx.send(poise::CreateReply::default().embed(embed)).await?;
        }
        None => {
            ctx.say(format!("❌ Case #{} not found in this server.", id)).await?;
        }
    }

    Ok(())
}

/// Edit the reason for a specific case.
#[poise::command(slash_command, guild_only, description_localized("en-US", "Update the reason for a specific case."))]
pub async fn edit(
    ctx: Context<'_>,
    #[description = "The case ID"] id: i64,
    #[description = "The new reason"] reason: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let gid = ctx.guild_id().unwrap().get() as i64;

    let result = sqlx::query!(
        "UPDATE cases SET reason = ? WHERE id = ? AND guild_id = ?",
        reason, id, gid
    ).execute(&ctx.data().db).await?;

    if result.rows_affected() == 0 {
        ctx.say(format!("❌ Case #{} not found.", id)).await?;
    } else {
        ctx.say(format!("✅ Case #{} reason has been updated.", id)).await?;
    }

    Ok(())
}

/// Delete a specific case (Permanent!).
#[poise::command(slash_command, guild_only, description_localized("en-US", "Delete a case permanently from the record."))]
pub async fn delete(
    ctx: Context<'_>,
    #[description = "The case ID"] id: i64,
) -> Result<(), Error> {
    ctx.defer().await?;
    let gid = ctx.guild_id().unwrap().get() as i64;

    let result = sqlx::query!(
        "DELETE FROM cases WHERE id = ? AND guild_id = ?",
        id, gid
    ).execute(&ctx.data().db).await?;

    if result.rows_affected() == 0 {
        ctx.say(format!("❌ Case #{} not found.", id)).await?;
    } else {
        ctx.say(format!("🗑️ Case #{} has been deleted.", id)).await?;
    }

    Ok(())
}
