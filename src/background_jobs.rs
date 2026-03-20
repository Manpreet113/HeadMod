use poise::serenity_prelude as serenity;
use std::sync::Arc;
use crate::types::Data;
use chrono::{Datelike, Timelike};

pub async fn start_background_jobs(data: Arc<Data>, http: Arc<serenity::Http>) {
    let d1 = data.clone();
    let h1 = http.clone();
    tokio::spawn(async move { temp_role_worker(d1, h1).await });

    let d2 = data.clone();
    let h2 = http.clone();
    tokio::spawn(async move { weekly_summary_worker(d2, h2).await });
}

async fn temp_role_worker(data: Arc<Data>, http: Arc<serenity::Http>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
    loop {
        interval.tick().await;
        let now = chrono::Utc::now();
        
        let expired = sqlx::query!(
            "SELECT id, guild_id, user_id, role_id FROM temporary_roles WHERE expires_at <= ?",
            now
        ).fetch_all(&data.db).await.unwrap_or_default();

        for row in expired {
            let gid = serenity::GuildId::new(row.guild_id as u64);
            let uid = serenity::UserId::new(row.user_id as u64);
            let rid = serenity::RoleId::new(row.role_id as u64);

            if let Ok(member) = gid.member(&http, uid).await {
                let _ = member.remove_role(&http, rid).await;
                
                if let Ok(bot) = http.get_current_user().await {
                    crate::logging::log_mod_action(
                        &http, &data, gid, &bot.into(), &member.user,
                        crate::logging::ModAction::Unban { reason: "Temporary role expired" }
                    ).await;
                }
            }

            let _ = sqlx::query!("DELETE FROM temporary_roles WHERE id = ?", row.id).execute(&data.db).await;
        }
    }
}

async fn weekly_summary_worker(data: Arc<Data>, http: Arc<serenity::Http>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
    loop {
        interval.tick().await;
        let now = chrono::Utc::now();
        
        if now.weekday() == chrono::Weekday::Sun && now.hour() == 4 {
            let configs = sqlx::query!("SELECT guild_id, mod_log_channel_id, last_summary_at FROM guild_configs").fetch_all(&data.db).await.unwrap_or_default();
            
            for config in configs {
                let last_summary = config.last_summary_at;
                let gid_val = config.guild_id.unwrap_or(0);
                if gid_val == 0 { continue; }
                
                let should_send = match last_summary {
                    Some(dt) => dt.date() != now.date_naive(),
                    None => true,
                };

                if should_send {
                    if let Some(log_id) = config.mod_log_channel_id {
                        let log_chan = serenity::ChannelId::new(log_id as u64);
                        if let Ok(summary) = generate_summary(&data, gid_val).await {
                            let _ = log_chan.send_message(&http, summary).await;
                            let _ = sqlx::query!(
                                "UPDATE guild_configs SET last_summary_at = CURRENT_TIMESTAMP WHERE guild_id = ?",
                                gid_val
                            ).execute(&data.db).await;
                        }
                    }
                }
            }
        }
    }
}

async fn generate_summary(data: &Data, guild_id: i64) -> Result<serenity::CreateMessage, crate::Error> {
    let seven_days_ago = chrono::Utc::now() - chrono::Duration::days(7);
    
    let cases = sqlx::query!(
        "SELECT action_type, COUNT(*) as \"count!: i64\" FROM cases WHERE guild_id = ? AND created_at >= ? GROUP BY action_type",
        guild_id, seven_days_ago
    ).fetch_all(&data.db).await?;

    let reports = sqlx::query!(
        "SELECT COUNT(*) as \"count!: i64\" FROM reports WHERE guild_id = ? AND created_at >= ?",
        guild_id, seven_days_ago
    ).fetch_one(&data.db).await?;

    let mut desc = format!("📊 **Server Moderation Health Report**\n_Last 7 days (since {}: UTC)_\n\n", seven_days_ago.format("%Y-%m-%d"));
    
    if cases.is_empty() {
        desc.push_str("• No moderation actions taken this week. (Peaceful!)\n");
    } else {
        for row in cases {
            desc.push_str(&format!("• **{}**: {} instances\n", row.action_type.to_uppercase(), row.count));
        }
    }
    
    desc.push_str(&format!("\n• **User Reports**: {} pending or resolved\n", reports.count));

    let embed = serenity::CreateEmbed::new()
        .title("Weekly Security Insight")
        .description(desc)
        .colour(serenity::Colour::BLURPLE)
        .footer(serenity::CreateEmbedFooter::new("Head Mod Security Summary"))
        .timestamp(serenity::Timestamp::now());

    Ok(serenity::CreateMessage::new().embed(embed))
}
