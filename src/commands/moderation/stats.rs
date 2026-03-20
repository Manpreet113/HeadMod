use poise::serenity_prelude as serenity;
use crate::{Context, Error};

/// View staff performance statistics and server moderation insights.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MODERATE_MEMBERS",
    rename = "performance"
)]
pub async fn stats(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let gid = ctx.guild_id().unwrap().get() as i64;
    let db = &ctx.data().db;

    // 1. Moderator Leaderboard (Total Cases)
    let leaderboard = sqlx::query!(
        "SELECT moderator_id, COUNT(*) as \"case_count!: i64\"
         FROM cases 
         WHERE guild_id = ? 
         GROUP BY moderator_id 
         ORDER BY COUNT(*) DESC 
         LIMIT 5",
        gid
    ).fetch_all(db).await?;

    // 2. Report Stats & Response Time
    let report_stats = sqlx::query!(
        "SELECT 
            COUNT(*) as \"total!: i64\",
            COUNT(resolved_at) as \"resolved!: i64\",
            AVG(unixepoch(resolved_at) - unixepoch(created_at)) as avg_response_secs
         FROM reports 
         WHERE guild_id = ?",
        gid
    ).fetch_one(db).await?;

    let mut embed = serenity::CreateEmbed::new()
        .title("📈 Staff Performance & Insights")
        .colour(serenity::Colour::BLURPLE)
        .timestamp(serenity::Timestamp::now());

    // Build Leaderboard string
    let mut leaderboard_str = String::new();
    for (i, row) in leaderboard.iter().enumerate() {
        let emoji = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "👤",
        };
        leaderboard_str.push_str(&format!("{} <@{}>: **{}** actions\n", emoji, row.moderator_id, row.case_count));
    }
    if leaderboard_str.is_empty() {
        leaderboard_str = "_No moderation actions recorded yet._".to_string();
    }

    embed = embed.field("🏆 Moderator Leaderboard", leaderboard_str, false);

    // Build Response Time string
    let avg_time = if let Some(secs_f) = report_stats.avg_response_secs {
        let secs = secs_f as i64;
        if secs < 60 {
            format!("{} seconds", secs)
        } else if secs < 3600 {
            format!("{:.1} minutes", (secs as f64) / 60.0)
        } else {
            format!("{:.1} hours", (secs as f64) / 3600.0)
        }
    } else {
        "N/A".to_string()
    };

    let metrics_str = format!(
        "• **Total reports:** {}\n• **Resolved:** {}\n• **Avg. Response Time:** `{}`",
        report_stats.total, report_stats.resolved, avg_time
    );

    embed = embed.field("⏱️ Efficiency Metrics", metrics_str, false);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
