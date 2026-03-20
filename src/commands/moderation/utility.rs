use poise::serenity_prelude as serenity;
use crate::{Context, Error};

/// Set channel slowmode.
#[poise::command(slash_command, guild_only, default_member_permissions = "MANAGE_MESSAGES")]
pub async fn slowmode(
    ctx: Context<'_>,
    #[description = "Slowmode duration in seconds (0 to disable)"] seconds: u64,
) -> Result<(), Error> {
    ctx.defer().await?;
    
    let edit = serenity::EditChannel::new().rate_limit_per_user(seconds as u16);
    ctx.channel_id().edit(&ctx.http(), edit).await?;
    
    if seconds == 0 {
        ctx.say("✅ Slowmode has been disabled.").await?;
    } else {
        ctx.say(format!("✅ Slowmode set to **{} seconds**.", seconds)).await?;
    }
    
    Ok(())
}

/// Bulk delete messages.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MANAGE_MESSAGES",
    description_localized("en-US", "Bulk delete messages from the current channel.")
)]
pub async fn purge(
    ctx: Context<'_>,
    #[description = "Number of messages to delete (max 100)"] 
    #[min = 1] #[max = 100] count: u32,
    #[description = "Only delete messages from this user"] user: Option<serenity::User>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let messages = ctx.channel_id().messages(&ctx.http(), serenity::GetMessages::new().limit(count as u8)).await?;
    
    let now = chrono::Utc::now().timestamp();
    let two_weeks_ago = now - (14 * 24 * 3600);

    let mut ids_to_delete = Vec::new();
    for msg in messages {
        if msg.timestamp.unix_timestamp() < two_weeks_ago { continue; }
        if let Some(ref target) = user {
            if msg.author.id != target.id { continue; }
        }
        ids_to_delete.push(msg.id);
    }

    if ids_to_delete.is_empty() {
        ctx.say("❌ No messages found to delete (or they are older than 14 days).").await?;
        return Ok(());
    }

    let deleted_count = ids_to_delete.len();
    ctx.channel_id().delete_messages(&ctx.http(), ids_to_delete).await?;

    ctx.say(format!("✅ Deleted **{}** message(s).", deleted_count)).await?;
    
    // Log the purge
    if let Some(config) = ctx.data().get_config(ctx.guild_id().unwrap()).await {
        if let Some(chan_id) = config.message_log_channel_id {
            let log_chan = serenity::ChannelId::new(chan_id as u64);
            let mut embed = serenity::CreateEmbed::new()
                .title("🧹 Messages Purged")
                .colour(serenity::Colour::TEAL)
                .field("Moderator", ctx.author().name.clone(), true)
                .field("Channel", format!("<#{}>", ctx.channel_id()), true)
                .field("Count", deleted_count.to_string(), true)
                .timestamp(serenity::Timestamp::now());
            
            if let Some(ref target) = user {
                embed = embed.field("Filter", format!("User: <@{}>", target.id), false);
            }
            
            let _ = log_chan.send_message(&ctx.http(), serenity::CreateMessage::new().embed(embed)).await;
        }
    }

    Ok(())
}
