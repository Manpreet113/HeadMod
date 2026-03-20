use poise::serenity_prelude as serenity;
use crate::events::message_log::DataKey;

pub struct MemberHandler;

#[serenity::async_trait]
impl serenity::EventHandler for MemberHandler {
    async fn guild_member_addition(&self, ctx: serenity::Context, new_member: serenity::Member) {
        let data = ctx.data.read().await;
        let Some(bot_data) = data.get::<DataKey>() else { return };
        
        let config = bot_data.get_config(new_member.guild_id).await;
        let Some(config) = config else { return };

        if let Some(chan_id) = config.join_log_channel_id {
            let log_chan = serenity::ChannelId::new(chan_id as u64);
            let embed = serenity::CreateEmbed::new()
                .title("📥 Member Joined")
                .colour(serenity::Colour::DARK_GREEN)
                .thumbnail(new_member.user.avatar_url().unwrap_or_default())
                .description(format!("**{}** ({}) joined the server.", new_member.user.tag(), new_member.user.id))
                .field("Account Created", new_member.user.created_at().to_string(), false);
            
            let _ = log_chan.send_message(&ctx.http, serenity::CreateMessage::new().embed(embed)).await;
        }

        let mut suspicious_reasons: Vec<String> = Vec::new();
        let ts = new_member.user.created_at();
        let created_at = chrono::DateTime::from_timestamp(ts.unix_timestamp(), 0).unwrap_or_default();
        let account_age = chrono::Utc::now().signed_duration_since(created_at);
        if account_age.num_hours() < 24 {
            suspicious_reasons.push("Fresh out of the oven (Account < 24h old)".to_string());
        }

        if config.global_intel_enabled {
            let user_id = new_member.user.id.get() as i64;
            let global_ban = sqlx::query!(
                "SELECT reason FROM global_bans WHERE user_id = ?",
                user_id
            ).fetch_optional(&bot_data.db).await.unwrap_or(None);

            if let Some(row) = global_ban {
                suspicious_reasons.push(format!("Global Fugitive: Recorded history of '{}'", row.reason));
            }
        }

        if !suspicious_reasons.is_empty() {
            if let Some(alert_chan_id) = config.suspicious_log_channel_id.or(config.mod_log_channel_id) {
                let alert_chan = serenity::ChannelId::new(alert_chan_id as u64);
                
                let quips = vec![
                    "This account is so new it still thinks the 'Wumpus' is a mythical creature.",
                    "Warning: This user arrived with a risk level higher than my CPU temperature during a heavy compile.",
                    "We've got a live one! This account popped into existence roughly ten minutes after the morning coffee.",
                    "Global sensors are twitching. This user has a 'Reputation' that precedes them (and it's not the Taylor Swift kind).",
                    "Security alert: This member is fresh enough to be served in a salad. Handle with caution."
                ];
                
                let random_quip = quips[new_member.user.id.get() as usize % quips.len()];
                
                let embed = serenity::CreateEmbed::new()
                    .title("🕵️ SUSPICIOUS ARRIVAL")
                    .colour(serenity::Colour::RED)
                    .description(format!("**{}** just joined and triggered the security sensors.", new_member.user.tag()))
                    .field("Intelligence Report", suspicious_reasons.join("\n"), false)
                    .field("Audit Insight", random_quip, false)
                    .footer(serenity::CreateEmbedFooter::new("Head Mod Security Protocol"));
                
                let _ = alert_chan.send_message(&ctx.http, serenity::CreateMessage::new().embed(embed)).await;
            }
        }
    }

    async fn guild_member_removal(&self, ctx: serenity::Context, guild_id: serenity::GuildId, user: serenity::User, _member_data: Option<serenity::Member>) {
        let data = ctx.data.read().await;
        let Some(bot_data) = data.get::<DataKey>() else { return };
        
        let config = bot_data.get_config(guild_id).await;
        let Some(config) = config else { return };

        if let Some(chan_id) = config.leave_log_channel_id {
            let log_chan = serenity::ChannelId::new(chan_id as u64);
            let embed = serenity::CreateEmbed::new()
                .title("📤 Member Left")
                .colour(serenity::Colour::RED)
                .description(format!("**{}** ({}) has vanished from the server.", user.tag(), user.id));
            
            let _ = log_chan.send_message(&ctx.http, serenity::CreateMessage::new().embed(embed)).await;
        }
    }
}
