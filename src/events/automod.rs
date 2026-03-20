use poise::serenity_prelude as serenity;
use crate::events::message_log::DataKey;

pub struct AutomodHandler;

#[serenity::async_trait]
impl serenity::EventHandler for AutomodHandler {
    async fn message(&self, ctx: serenity::Context, new_message: serenity::Message) {
        if new_message.author.bot { return; }
        let guild_id = match new_message.guild_id {
            Some(gid) => gid,
            None => return,
        };

        if let Ok(member) = new_message.member(&ctx.http).await {
            #[allow(deprecated)]
            if let Ok(perms) = member.permissions(&ctx.cache) {
                if perms.contains(serenity::Permissions::MANAGE_MESSAGES) || perms.contains(serenity::Permissions::ADMINISTRATOR) {
                    return;
                }
            }
        }

        let data = ctx.data.read().await;
        let Some(bot_data) = data.get::<DataKey>() else { return };

        let gid_i64 = guild_id.get() as i64;
        let config = bot_data.get_config(guild_id).await;

        let (anti_invite, anti_spam, anti_caps) = match config {
            Some(ref c) => (c.anti_invite, c.anti_spam, c.anti_caps),
            None => (true, true, false),
        };

        let blacklisted_words = sqlx::query!(
            "SELECT word FROM blacklisted_words WHERE guild_id = ?",
            gid_i64
        ).fetch_all(&bot_data.db).await.unwrap_or_default();

        let content_lower = new_message.content.to_lowercase();
        for row in blacklisted_words {
            if content_lower.contains(&row.word) {
                let _ = new_message.delete(&ctx.http).await;
                let _ = new_message.channel_id.say(&ctx.http, format!("⚠️ <@{}>, your message contained a blacklisted word and was removed.", new_message.author.id)).await;
                return;
            }
        }

        if anti_invite && (new_message.content.contains("discord.gg/") || new_message.content.contains("discord.com/invite/")) {
            let _ = new_message.delete(&ctx.http).await;
            let _ = new_message.channel_id.say(&ctx.http, format!("⚠️ <@{}>, please do not post invite links.", new_message.author.id)).await;
            return;
        }

        if anti_spam {
            let mut trigger_timeout = false;
            {
                let mut counts = bot_data.message_counts.entry(new_message.author.id).or_insert_with(Vec::new);
                let now = chrono::Utc::now();
                counts.retain(|t| now.signed_duration_since(*t).num_seconds() < 5);
                counts.push(now);
                
                if counts.len() > 5 {
                    counts.clear();
                    trigger_timeout = true;
                }
            }

            if trigger_timeout {
                let now = chrono::Utc::now();
                let until = now + chrono::Duration::minutes(5);
                if let Ok(ts) = serenity::Timestamp::from_unix_timestamp(until.timestamp()) {
                    let edit = serenity::EditMember::new().disable_communication_until(ts.to_string());
                    if let Ok(_) = guild_id.edit_member(&ctx.http, new_message.author.id, edit).await {
                        let _ = new_message.delete(&ctx.http).await;
                        
                        if let Ok(bot) = ctx.http.get_current_user().await {
                            crate::logging::log_mod_action(
                                &ctx.http, &bot_data, guild_id, &bot.into(), &new_message.author,
                                crate::logging::ModAction::Timeout { reason: "Automod: Message flood", duration: "5m" }
                            ).await;
                        }
                        
                        let _ = new_message.channel_id.say(&ctx.http, format!("⏱️ <@{}> has been timed out for 5m. Reason: Message flood.", new_message.author.id)).await;
                    }
                }
            }
        }

        let total_mentions = new_message.mentions.len() + new_message.mention_roles.len();
        if total_mentions > 5 {
            let _ = new_message.delete(&ctx.http).await;
            let _ = new_message.channel_id.say(&ctx.http, format!("⚠️ <@{}>, please avoid mass-mentioning.", new_message.author.id)).await;
            return;
        }

        if anti_caps && new_message.content.len() > 10 {
            let uppercase_count = new_message.content.chars().filter(|c| c.is_uppercase()).count();
            let letter_count = new_message.content.chars().filter(|c| c.is_alphabetic()).count();

            if letter_count > 0 && (uppercase_count as f32 / letter_count as f32) > 0.7 {
                let _ = new_message.delete(&ctx.http).await;
                let _ = new_message.channel_id.say(&ctx.http, format!("⚠️ <@{}>, please turn off caps lock.", new_message.author.id)).await;
                return;
            }
        }

        let channel_config = bot_data.get_channel_config(new_message.channel_id).await;
        
        let (effective_threshold, is_relaxed) = match (config, channel_config) {
            (_, Some(cc)) if cc.toxicity_threshold > 0 => (cc.toxicity_threshold, cc.is_relaxed),
            (Some(gc), Some(cc)) => (gc.toxicity_threshold, cc.is_relaxed),
            (Some(gc), None) => (gc.toxicity_threshold, false),
            _ => (0, false),
        };

        if effective_threshold > 0 {
            let mut score = 0;
            let content = new_message.content.to_lowercase();
            
            let severe = vec!["nigger", "faggot", "kys", "retard", "rape"];
            for word in severe {
                if content.contains(word) { score += 50; }
            }

            let insults = vec!["bitch", "cunt", "asshole", "fuck you", "dick", "shutup"];
            for word in insults {
                if content.contains(word) { score += 20; }
            }

            let mut max_repeat = 0;
            let mut current_repeat = 1;
            let chars: Vec<char> = content.chars().collect();
            for i in 1..chars.len() {
                if chars[i] == chars[i-1] && chars[i].is_alphabetic() {
                    current_repeat += 1;
                } else {
                    max_repeat = max_repeat.max(current_repeat);
                    current_repeat = 1;
                }
            }
            if max_repeat > 6 { score += 15; }

            if is_relaxed {
                score /= 2;
            }

            if score >= effective_threshold {
                let _ = new_message.delete(&ctx.http).await;
                let _ = new_message.channel_id.say(&ctx.http, format!("⚠️ <@{}>, your message was flagged for high toxicity and removed.", new_message.author.id)).await;
                
                if let Ok(bot) = ctx.http.get_current_user().await {
                    crate::logging::log_mod_action(
                        &ctx.http, &bot_data, guild_id, &bot.into(), &new_message.author,
                        crate::logging::ModAction::Warn { reason: "Automod: Toxic content detected", warn_count: 0, auto_timeout: false }
                    ).await;
                }
                return;
            }
        }
    }
}
