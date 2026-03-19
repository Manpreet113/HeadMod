use chrono::Utc;
use poise::serenity_prelude as serenity;
use crate::{Context, Error};

/// Bulk-delete messages in this channel (max 100, must be under 14 days old).
#[poise::command(
    slash_command, guild_only,
    default_member_permissions = "MANAGE_MESSAGES",
    required_bot_permissions   = "MANAGE_MESSAGES"
)]
pub async fn purge(
    ctx: Context<'_>,
    #[description = "Number of messages to delete (1–100)"] amount: u8,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let amount     = amount.clamp(1, 100);
    let channel_id = ctx.channel_id();

    let messages = channel_id
        .messages(&ctx.http(), serenity::GetMessages::new().limit(amount + 1))
        .await?;

    let cutoff = Utc::now() - chrono::Duration::days(14);
    let ids: Vec<serenity::MessageId> = messages
        .iter()
        .filter(|m| m.timestamp.unix_timestamp() > cutoff.timestamp())
        .map(|m| m.id)
        .take(amount as usize)
        .collect();

    if ids.is_empty() {
        ctx.say("❌ No messages found that can be bulk-deleted (all are older than 14 days).").await?;
        return Ok(());
    }

    let deleted = ids.len();
    channel_id.delete_messages(&ctx.http(), &ids).await?;
    ctx.say(format!("🗑️ Deleted **{}** message(s).", deleted)).await?;

    Ok(())
}