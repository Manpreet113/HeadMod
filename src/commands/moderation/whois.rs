use poise::serenity_prelude as serenity;
use crate::{Context, Error};

/// Get detailed user information, including account age and infractions.
#[poise::command(slash_command, guild_only, default_member_permissions = "MODERATE_MEMBERS")]
pub async fn whois(
    ctx: Context<'_>,
    #[description = "The user to check"] user: serenity::User,
) -> Result<(), Error> {
    ctx.defer().await?;

    let guild_id = ctx.guild_id().ok_or("Could not fetch guild")?;
    let member = guild_id.member(&ctx.http(), user.id).await.ok();
    
    let gid = guild_id.get() as i64;
    let tid = user.id.get() as i64;
    
    // Fetch recent cases from DB
    let cases = sqlx::query!(
        "SELECT id, action_type, reason, created_at FROM cases WHERE guild_id = ? AND target_id = ? ORDER BY created_at DESC LIMIT 5",
        gid, tid
    ).fetch_all(&ctx.data().db).await?;

    let total_cases = sqlx::query!(
        "SELECT COUNT(*) as count FROM cases WHERE guild_id = ? AND target_id = ?",
        gid, tid
    ).fetch_one(&ctx.data().db).await?.count;

    let (risk_level, risk_colour) = match total_cases {
        0 => ("Safe", serenity::Colour::DARK_GREEN),
        1..=2 => ("Low", serenity::Colour::GOLD),
        3..=5 => ("Medium", serenity::Colour::ORANGE),
        _ => ("High", serenity::Colour::RED),
    };

    let mut embed = serenity::CreateEmbed::new()
        .title(format!("User Profile: {}", user.tag()))
        .thumbnail(user.face())
        .colour(risk_colour)
        .field("User ID", user.id.to_string(), true)
        .field("Risk Level", format!("**{}**", risk_level), true)
        .field("Account Created", format!("<t:{}:F>\n(<t:{}:R>)", user.created_at().unix_timestamp(), user.created_at().unix_timestamp()), false);

    if let Some(m) = member {
        if let Some(joined) = m.joined_at {
            embed = embed.field("Joined Server", format!("<t:{}:F>\n(<t:{}:R>)", joined.unix_timestamp(), joined.unix_timestamp()), false);
        }
        
        let roles = m.roles.iter()
            .map(|r| format!("<@&{}>", r.get()))
            .collect::<Vec<_>>();
            
        if !roles.is_empty() {
             embed = embed.field(format!("Roles [{}]", roles.len()), roles.join(" "), false);
        }
    }

    if !cases.is_empty() {
        let history_lines: Vec<String> = cases.iter().map(|c| {
            format!("`#{}` **{}**: {} (<t:{}:R>)", c.id.unwrap_or_default(), c.action_type.to_uppercase(), c.reason, c.created_at.and_utc().timestamp())
        }).collect();
        embed = embed.field(format!("Recent Infractions (Total: {})", total_cases), history_lines.join("\n"), false);
    } else {
        embed = embed.field("Infractions", "✅ No recorded cases.", false);
    }

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
