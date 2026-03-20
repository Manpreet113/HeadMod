use poise::serenity_prelude as serenity;
use crate::{Context, Error};

/// Report a message to the moderators.
#[poise::command(context_menu_command = "Report Message", guild_only)]
pub async fn report_message(
    ctx: Context<'_>,
    #[description = "The message to report"] message: serenity::Message,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    
    let guild_id = ctx.guild_id().ok_or("Must be in a guild")?;
    let reporter = ctx.author();
    let target = &message.author;
    
    if target.id == reporter.id {
        ctx.say("❌ You cannot report your own message.").await?;
        return Ok(());
    }

    let gid = guild_id.get() as i64;
    let rid = reporter.id.get() as i64;
    let tid = target.id.get() as i64;
    let mid = message.id.get() as i64;
    let cid = message.channel_id.get() as i64;
    let content = &message.content;

    // Save to DB
    sqlx::query!(
        "INSERT INTO reports (guild_id, reporter_id, target_id, message_id, channel_id, content) VALUES (?, ?, ?, ?, ?, ?)",
        gid, rid, tid, mid, cid, content
    ).execute(&ctx.data().db).await?;

    // Send to mod log
    let config = ctx.data().get_config(guild_id).await;
    if let Some(log_cid) = config.and_then(|c| c.mod_log_channel_id) {
        let channel = serenity::ChannelId::new(log_cid as u64);
        
        let embed = serenity::CreateEmbed::new()
            .title("🚩 New Message Report")
            .colour(serenity::Colour::RED)
            .field("Reporter", format!("<@{}>", reporter.id), true)
            .field("Target", format!("<@{}>", target.id), true)
            .field("Channel", format!("<#{}>", message.channel_id), true)
            .field("Message Content", content, false)
            .field("Link", format!("[Jump to Message](https://discord.com/channels/{}/{}/{})", guild_id, message.channel_id, message.id), false)
            .timestamp(serenity::Timestamp::now());

        let row = serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new(format!("report_warn_{}", mid)).label("Warn User").style(serenity::ButtonStyle::Secondary),
            serenity::CreateButton::new(format!("report_delete_{}", mid)).label("Delete Msg").style(serenity::ButtonStyle::Danger),
            serenity::CreateButton::new(format!("report_ignore_{}", mid)).label("Ignore").style(serenity::ButtonStyle::Secondary),
        ]);

        let _ = channel.send_message(&ctx.http(), serenity::CreateMessage::new().embed(embed).components(vec![row])).await;
    }

    ctx.say("✅ Thank you! Your report has been sent to the moderators.").await?;
    Ok(())
}
