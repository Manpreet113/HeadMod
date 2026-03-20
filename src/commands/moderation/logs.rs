use poise::serenity_prelude as serenity;
use crate::{Context, Error};

/// View a user's deleted/edited message history.
#[poise::command(
    slash_command, guild_only,
    default_member_permissions = "MODERATE_MEMBERS",
    description_localized("en-US", "View the archived message history for a specific user.")
)]
pub async fn logs(
    ctx: Context<'_>,
    #[description = "The user to check"] user: serenity::User,
) -> Result<(), Error> {
    ctx.defer().await?;

    let gid = ctx.guild_id().unwrap().get() as i64;
    let tid = user.id.get() as i64;

    let rows = sqlx::query!(
        "SELECT channel_id, content, action_type, created_at FROM message_logs 
         WHERE guild_id = ? AND user_id = ? ORDER BY id DESC LIMIT 10",
        gid, tid
    ).fetch_all(&ctx.data().db).await?;

    if rows.is_empty() {
        ctx.say(format!("✅ No archived logs found for **{}**.", user.name)).await?;
    } else {
        let lines: Vec<String> = rows.into_iter().map(|r| {
            let action = if r.action_type == "delete" { "🗑️ DELETE" } else { "✏️ EDIT" };
            format!("**{}** in <#{}>: {}\n_(<t:{}:R>)_", action, r.channel_id, r.content, r.created_at.and_utc().timestamp())
        }).collect();

        let embed = serenity::CreateEmbed::new()
            .title(format!("Audit Logs for {}", user.name))
            .description(lines.join("\n\n"))
            .colour(serenity::Colour::MEIBE_PINK);

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    }

    Ok(())
}
