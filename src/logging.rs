use poise::serenity_prelude as serenity;
use crate::Data;

// ── Mod action logging ────────────────────────────────────────────────────────

/// The kind of moderation action being logged.
pub enum ModAction<'a> {
    Kick { reason: &'a str },
    Ban  { reason: &'a str, delete_days: u8 },
    Unban,
}

/// Post a mod action embed to the mod log channel.
///
/// Failures are logged with `tracing::error!` and swallowed — a logging
/// failure should never cause the command itself to return an error.
pub async fn log_mod_action(
    http: &serenity::Http,
    data: &Data,
    moderator: &serenity::User,
    target: &serenity::User,
    action: ModAction<'_>,
) {
    let (title, colour, extra) = match action {
        ModAction::Kick { reason } => (
            "👢 Member Kicked",
            serenity::Colour::ORANGE,
            format!("**Reason:** {}", reason),
        ),
        ModAction::Ban { reason, delete_days } => (
            "🔨 Member Banned",
            serenity::Colour::RED,
            format!("**Reason:** {}\n**Messages deleted:** {} day(s)", reason, delete_days),
        ),
        ModAction::Unban => (
            "✅ Member Unbanned",
            serenity::Colour::DARK_GREEN,
            String::new(),
        ),
    };

    let embed = serenity::CreateEmbed::new()
        .title(title)
        .colour(colour)
        .field("Target",    format!("{} (`{}`)", target.name,    target.id),    false)
        .field("Moderator", format!("{} (`{}`)", moderator.name, moderator.id), false)
        .description(extra)
        .timestamp(serenity::Timestamp::now());

    let msg = serenity::CreateMessage::new().embed(embed);

    if let Err(e) = data.mod_log_channel.send_message(http, msg).await {
        tracing::error!("Failed to send mod log: {}", e);
    }
}

// ── Message event logging ─────────────────────────────────────────────────────

/// Serenity event handler — handles message delete/edit events and posts
/// them to the message log channel.
pub struct LogHandler;

#[serenity::async_trait]
impl serenity::EventHandler for LogHandler {
    /// Fired when a cached message is deleted.
    ///
    /// We can only log content here if the message was in serenity's cache
    /// (i.e. received while the bot was running). There's no way to recover
    /// content for uncached messages — we log what we can.
    async fn message_delete(
        &self,
        ctx: serenity::Context,
        channel_id: serenity::ChannelId,
        deleted_message_id: serenity::MessageId,
        _guild_id: Option<serenity::GuildId>,
    ) {
        let data = ctx.data.read().await;
        let Some(bot_data) = data.get::<DataKey>() else { return };

        // Extract owned data from the cache *before* any .await.
        // ctx.cache.message() returns an Option<CacheRef> which is !Send,
        // so we must fully consume it into plain owned types here.
        struct CachedMsg {
            author_name: String,
            author_id:   serenity::UserId,
            author_bot:  bool,
            content:     String,
        }

        let cached: Option<CachedMsg> = ctx.cache.message(channel_id, deleted_message_id)
            .map(|msg| CachedMsg {
                author_name: msg.author.name.clone(),
                author_id:   msg.author.id,
                author_bot:  msg.author.bot,
                content:     msg.content.clone(),
            });
        // CacheRef is dropped here — only plain owned data remains.

        // Don't log the bot's own messages.
        if cached.as_ref().is_some_and(|m| m.author_bot) { return; }

        let embed = match cached {
            Some(msg) => {
                serenity::CreateEmbed::new()
                    .title("🗑️ Message Deleted")
                    .colour(serenity::Colour::RED)
                    .field("Author",  format!("{} (`{}`)", msg.author_name, msg.author_id), false)
                    .field("Channel", format!("<#{}>", channel_id), false)
                    .description(if msg.content.is_empty() {
                        "_No text content (may have been an embed or attachment)_".to_owned()
                    } else {
                        msg.content
                    })
                    .timestamp(serenity::Timestamp::now())
            }
            None => {
                serenity::CreateEmbed::new()
                    .title("🗑️ Message Deleted (not cached)")
                    .colour(serenity::Colour::RED)
                    .field("Channel",    format!("<#{}>", channel_id),   false)
                    .field("Message ID", deleted_message_id.to_string(), false)
                    .timestamp(serenity::Timestamp::now())
            }
        };

        let msg = serenity::CreateMessage::new().embed(embed);
        if let Err(e) = bot_data.message_log_channel.send_message(&ctx.http, msg).await {
            tracing::error!(
                channel = %bot_data.message_log_channel,
                error = %e,
                "Failed to send message delete log"
            );
        }
    }

    /// Fired when a cached message is edited.
    async fn message_update(
        &self,
        ctx: serenity::Context,
        old_if_available: Option<serenity::Message>,
        new: Option<serenity::Message>,
        event: serenity::MessageUpdateEvent,
    ) {
        let data = ctx.data.read().await;
        let Some(bot_data) = data.get::<DataKey>() else { return };

        // Only log if we have both old and new content to diff.
        let (Some(old_msg), Some(new_msg)) = (old_if_available, new) else { return };

        // Ignore bots and cases where the content didn't actually change
        // (e.g. an embed was just resolved by Discord).
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
        if let Err(e) = bot_data.message_log_channel.send_message(&ctx.http, msg).await {
            tracing::error!(
                channel = %bot_data.message_log_channel,
                error = %e,
                "Failed to send message edit log"
            );
        }
    }
}

// ── TypeMap key ───────────────────────────────────────────────────────────────

/// Key used to store `Data` in serenity's `TypeMap`, making it accessible
/// inside the raw `EventHandler` where poise's `Context` isn't available.
pub struct DataKey;

impl serenity::prelude::TypeMapKey for DataKey {
    type Value = std::sync::Arc<Data>;
}