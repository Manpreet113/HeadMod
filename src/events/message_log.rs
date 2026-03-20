use poise::serenity_prelude as serenity;
use crate::types::Data;

/// Key for storing `Data` in serenity's TypeMap so `EventHandler`
/// implementations can access it without poise's `Context`.
pub struct DataKey;

impl serenity::prelude::TypeMapKey for DataKey {
    type Value = std::sync::Arc<Data>;
}

/// Serenity event handler for message delete, edit, and guild event logging.
pub struct MessageLogHandler;

#[serenity::async_trait]
impl serenity::EventHandler for MessageLogHandler {
    async fn message_delete(
        &self,
        ctx: serenity::Context,
        channel_id: serenity::ChannelId,
        deleted_message_id: serenity::MessageId,
        guild_id_opt: Option<serenity::GuildId>,
    ) {
        let data = ctx.data.read().await;
        let Some(bot_data) = data.get::<DataKey>() else { return };

        struct CachedMsg {
            author_name: String,
            author_id:   serenity::UserId,
            author_bot:  bool,
            content:     String,
            attachments: Vec<serenity::Attachment>,
        }

        let cached = ctx.cache.message(channel_id, deleted_message_id)
            .map(|m| CachedMsg {
                author_name: m.author.name.clone(),
                author_id:   m.author.id,
                author_bot:  m.author.bot,
                content:     m.content.clone(),
                attachments: m.attachments.clone(),
            });

        if cached.as_ref().is_some_and(|m| m.author_bot) { return; }

        let embed = match cached {
            Some(ref m) => {
                let mut e = serenity::CreateEmbed::new()
                    .title("🗑️ Message Deleted")
                    .colour(serenity::Colour::RED)
                    .field("Author",  format!("{} (`{}`)", m.author_name, m.author_id), false)
                    .field("Channel", format!("<#{}>", channel_id), false)
                    .field("Attachments", format!("{}", m.attachments.len()), true)
                    .description(if m.content.is_empty() {
                        "_No text content (may have been an embed or attachment)_".to_owned()
                    } else {
                        m.content.clone()
                    })
                    .timestamp(serenity::Timestamp::now());
                if !m.attachments.is_empty() {
                    e = e.footer(serenity::CreateEmbedFooter::new("Media mirrored to Evidence Vault"));
                }
                e
            },
            None => serenity::CreateEmbed::new()
                .title("🗑️ Message Deleted (not cached)")
                .colour(serenity::Colour::RED)
                .field("Channel",    format!("<#{}>", channel_id),   false)
                .field("Message ID", deleted_message_id.to_string(), false)
                .timestamp(serenity::Timestamp::now()),
        };

        let msg = serenity::CreateMessage::new().embed(embed);
        if let Some(gid) = guild_id_opt {
            if let Some(config) = bot_data.get_config(gid).await {
                // 1. Mod Log
                if let Some(chan_id) = config.message_log_channel_id {
                    let log_chan = serenity::ChannelId::new(chan_id as u64);
                    let _ = log_chan.send_message(&ctx.http, msg).await;
                }

                // 2. Evidence Vault (Phase 8)
                if let (Some(ev_id), Some(ref m)) = (config.evidence_channel_id, cached.as_ref()) {
                    if !m.attachments.is_empty() {
                        let ev_chan = serenity::ChannelId::new(ev_id as u64);
                        let files: Vec<String> = m.attachments.iter().map(|a| a.proxy_url.clone()).collect();
                        let description = format!(
                            "🛡️ **Evidence for deleted message `{}`**\n**Author:** {} (`{}`)\n**Channel:** <#{}>\n**Content:** {}\n\n**Files:**\n{}",
                            deleted_message_id, m.author_name, m.author_id, channel_id, m.content, files.join("\n")
                        );
                        let _ = ev_chan.say(&ctx.http, description).await;
                    }
                }
            }
        }
    }

    async fn message_update(
        &self,
        ctx: serenity::Context,
        old_if_available: Option<serenity::Message>,
        new: Option<serenity::Message>,
        event: serenity::MessageUpdateEvent,
    ) {
        let data = ctx.data.read().await;
        let Some(bot_data) = data.get::<DataKey>() else { return };

        let (Some(old_msg), Some(new_msg)) = (old_if_available, new) else { return };
        if old_msg.author.bot { return; }
        if old_msg.content == new_msg.content { return; }

        let embed = serenity::CreateEmbed::new()
            .title("✏️ Message Edited")
            .colour(serenity::Colour::GOLD)
            .field("Author",  format!("{} (`{}`)", old_msg.author.name, old_msg.author.id), false)
            .field("Channel", format!("<#{}>", event.channel_id), false)
            .field("Before",  &old_msg.content, false)
            .field("After",   &new_msg.content, false)
            .field("Jump",    format!("[Go to message]({})", new_msg.link()), false)
            .timestamp(serenity::Timestamp::now());

        let msg = serenity::CreateMessage::new().embed(embed);
        if let Some(gid) = event.guild_id {
            let gid_i64 = gid.get() as i64;
            let cid_i64 = event.channel_id.get() as i64;
            let uid_i64 = old_msg.author.id.get() as i64;

            let _ = sqlx::query!(
                "INSERT INTO message_logs (guild_id, channel_id, user_id, content, action_type) VALUES (?, ?, ?, ?, 'edit')",
                gid_i64, cid_i64, uid_i64, old_msg.content
            ).execute(&bot_data.db).await;

            if let Some(config) = bot_data.get_config(gid).await {
                if let Some(chan_id) = config.message_log_channel_id {
                    let log_chan = serenity::ChannelId::new(chan_id as u64);
                    let _ = log_chan.send_message(&ctx.http, msg).await;
                }
            }
        }
    }

    async fn guild_member_addition(&self, ctx: serenity::Context, new_member: serenity::Member) {
        let data = ctx.data.read().await;
        let Some(bot_data) = data.get::<DataKey>() else { return };

        if let Some(config) = bot_data.get_config(new_member.guild_id).await {
            if let Some(chan_id) = config.message_log_channel_id {
                let log_chan = serenity::ChannelId::new(chan_id as u64);
                
                let mut embed = serenity::CreateEmbed::new()
                    .title("📥 Member Joined")
                    .colour(serenity::Colour::BLURPLE)
                    .thumbnail(new_member.user.face())
                    .field("User", format!("{} (`{}`)", new_member.user.name, new_member.user.id), false)
                    .field("Account Created", format!("<t:{}:R>", new_member.user.created_at().unix_timestamp()), false)
                    .timestamp(serenity::Timestamp::now());

                if config.verification_channel_id.is_some() {
                    embed = embed.field("Status", "⏳ Pending Verification", true);
                }
                
                let _ = log_chan.send_message(&ctx.http, serenity::CreateMessage::new().embed(embed)).await;

                // 2. Alt Detection
                if config.min_account_age_days > 0 {
                    let now = chrono::Utc::now().timestamp();
                    let account_created = new_member.user.created_at().unix_timestamp();
                    let account_age_days = (now - account_created) / 86400;

                    if account_age_days < config.min_account_age_days {
                        let _ = new_member.kick(&ctx.http).await;
                        let embed_alt = serenity::CreateEmbed::new()
                            .title("🛡️ Alt Account Kicked")
                            .colour(serenity::Colour::RED)
                            .description(format!(
                                "**{}** was kicked because their account age ({} days) is below the threshold ({} days).",
                                new_member.user.name, account_age_days, config.min_account_age_days
                            ))
                            .timestamp(serenity::Timestamp::now());
                        let _ = log_chan.send_message(&ctx.http, serenity::CreateMessage::new().embed(embed_alt)).await;
                    }
                }

                // 3. Raid Protection (10 joins in 60s)
                let mut trigger_raid = false;
                {
                    let mut counts = bot_data.join_counts.entry(new_member.guild_id).or_insert_with(Vec::new);
                    let now = chrono::Utc::now();
                    counts.retain(|t| now.signed_duration_since(*t).num_seconds() < 60);
                    counts.push(now);
                    if counts.len() > 10 {
                        trigger_raid = true;
                    }
                }

                if trigger_raid {
                    let embed_raid = serenity::CreateEmbed::new()
                        .title("🚨 RAID ALERT")
                        .colour(serenity::Colour::ORANGE)
                        .description("Multiple users are joining rapidly (>10 in 60s). Consider enabling `/lockdown`.")
                        .timestamp(serenity::Timestamp::now());
                    let _ = log_chan.send_message(&ctx.http, serenity::CreateMessage::new().embed(embed_raid)).await;
                }
            }
        }
    }

    async fn guild_member_removal(&self, ctx: serenity::Context, guild_id: serenity::GuildId, user: serenity::User, _: Option<serenity::Member>) {
        let data = ctx.data.read().await;
        let Some(bot_data) = data.get::<DataKey>() else { return };

        if let Some(config) = bot_data.get_config(guild_id).await {
            if let Some(chan_id) = config.message_log_channel_id {
                let log_chan = serenity::ChannelId::new(chan_id as u64);
                let embed = serenity::CreateEmbed::new()
                    .title("📤 Member Left")
                    .colour(serenity::Colour::RED)
                    .field("User", format!("{} (`{}`)", user.name, user.id), false)
                    .timestamp(serenity::Timestamp::now());
                
                let _ = log_chan.send_message(&ctx.http, serenity::CreateMessage::new().embed(embed)).await;
            }
        }
    }

    async fn guild_member_update(&self, ctx: serenity::Context, old_if_available: Option<serenity::Member>, new: Option<serenity::Member>, event: serenity::GuildMemberUpdateEvent) {
        let data = ctx.data.read().await;
        let Some(bot_data) = data.get::<DataKey>() else { return };

        let Some(new_member) = new else { return };
        let Some(old_member) = old_if_available else { return };

        if old_member.roles == new_member.roles { return; }

        if let Some(config) = bot_data.get_config(event.guild_id).await {
            if let Some(chan_id) = config.message_log_channel_id {
                let log_chan = serenity::ChannelId::new(chan_id as u64);
                
                let added: Vec<_> = new_member.roles.iter().filter(|r| !old_member.roles.contains(r)).map(|r| format!("<@&{}>", r.get())).collect();
                let removed: Vec<_> = old_member.roles.iter().filter(|r| !new_member.roles.contains(r)).map(|r| format!("<@&{}>", r.get())).collect();

                if added.is_empty() && removed.is_empty() { return; }

                let mut embed = serenity::CreateEmbed::new()
                    .title("🛡️ Roles Updated")
                    .colour(serenity::Colour::PURPLE)
                    .field("User", format!("{} (`{}`)", new_member.user.name, new_member.user.id), false)
                    .timestamp(serenity::Timestamp::now());

                if !added.is_empty() {
                    embed = embed.field("Added", added.join(", "), false);
                }
                if !removed.is_empty() {
                    embed = embed.field("Removed", removed.join(", "), false);
                }

                let _ = log_chan.send_message(&ctx.http, serenity::CreateMessage::new().embed(embed)).await;
            }
        }
    }
}