use poise::serenity_prelude as serenity;
use crate::{Context, Error};

/// Send a persistent moderation dashboard to the current channel.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MODERATE_MEMBERS",
    description_localized("en-US", "Display the interactive moderation command center.")
)]
pub async fn center(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let gid_i64 = guild_id.get() as i64;

    // Fetch some stats for the dashboard
    let case_count = sqlx::query!("SELECT COUNT(*) as count FROM cases WHERE guild_id = ?", gid_i64)
        .fetch_one(&ctx.data().db).await?.count;
    
    let report_count = sqlx::query!("SELECT COUNT(*) as count FROM reports WHERE guild_id = ? AND status = 'open'", gid_i64)
        .fetch_one(&ctx.data().db).await?.count;

    let embed = serenity::CreateEmbed::new()
        .title("🛡️ Head Mod Command Center")
        .description("Unified control panel for server moderation and oversight.")
        .field("📊 Stats", format!("**Total Cases:** {}\n**Open Reports:** {}", case_count, report_count), true)
        .field("🛡️ Status", "System: **Operational**\nMode: **Standard**", true)
        .colour(serenity::Colour::BLURPLE)
        .footer(serenity::CreateEmbedFooter::new(format!("Guild: {}", guild_id)));

    let row1 = serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new("center_cases").label("Recent Cases").style(serenity::ButtonStyle::Secondary).emoji('📝'),
        serenity::CreateButton::new("center_reports").label("View Reports").style(serenity::ButtonStyle::Secondary).emoji('🚩'),
        serenity::CreateButton::new("center_strikes").label("Strike Rules").style(serenity::ButtonStyle::Secondary).emoji('⚡'),
    ]);

    let row2 = serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new("center_lockdown").label("Lockdown Server").style(serenity::ButtonStyle::Danger).emoji('🔒'),
        serenity::CreateButton::new("center_refresh").label("Refresh").style(serenity::ButtonStyle::Primary).emoji('🔄'),
    ]);

    ctx.send(poise::CreateReply::default()
        .embed(embed)
        .components(vec![row1, row2])
    ).await?;

    Ok(())
}
