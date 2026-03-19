use poise::serenity_prelude as serenity;
use crate::types::Data;

/// Key for storing `Data` in serenity's TypeMap so `EventHandler`
/// implementations can access it without poise's `Context`.
pub struct DataKey;

impl serenity::prelude::TypeMapKey for DataKey {
    type Value = std::sync::Arc<Data>;
}

/// Serenity event handler for message delete and edit logging.
pub struct MessageLogHandler;

#[serenity::async_trait]
impl serenity::EventHandler for MessageLogHandler {
    async fn message_delete(
        &self,
        ctx: serenity::Context,
        channel_id: serenity::ChannelId,
        deleted_message_id: serenity::MessageId,
        _guild_id: Option<serenity::GuildId>,
    ) {
        let data = ctx.data.read().await;
        let Some(bot_data) = data.get::<DataKey>() else { return };

        struct CachedMsg {
            author_name: String,
            author_id:   serenity::UserId,
            author_bot:  bool,
            content:     String,
        }

        let cached = ctx.cache.message(channel_id, deleted_message_id)
            .map(|m| CachedMsg {
                author_name: m.author.name.clone(),
                author_id:   m.author.id,
                author_bot:  m.author.bot,
                content:     m.content.clone(),
            });

        if cached.as_ref().is_some_and(|m| m.author_bot) { return; }

        let embed = match cached {
            Some(m) => serenity::CreateEmbed::new()
                .title("🗑️ Message Deleted")
                .colour(serenity::Colour::RED)
                .field("Author",  format!("{} (`{}`)", m.author_name, m.author_id), false)
                .field("Channel", format!("<#{}>", channel_id), false)
                .description(if m.content.is_empty() {
                    "_No text content (may have been an embed or attachment)_".to_owned()
                } else {
                    m.content
                })
                .timestamp(serenity::Timestamp::now()),
            None => serenity::CreateEmbed::new()
                .title("🗑️ Message Deleted (not cached)")
                .colour(serenity::Colour::RED)
                .field("Channel",    format!("<#{}>", channel_id),   false)
                .field("Message ID", deleted_message_id.to_string(), false)
                .timestamp(serenity::Timestamp::now()),
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
        if let Err(e) = bot_data.message_log_channel.send_message(&ctx.http, msg).await {
            tracing::error!(
                channel = %bot_data.message_log_channel,
                error = %e,
                "Failed to send message edit log"
            );
        }
    }
}