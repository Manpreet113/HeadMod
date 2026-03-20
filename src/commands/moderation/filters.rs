use crate::types::{Context, Error};
use poise::serenity_prelude as serenity;

/// Set channel-specific message filters
#[poise::command(slash_command, guild_only, subcommands("set_threshold", "toggle_relaxed", "view_status"))]
pub async fn channel_filter(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Set a custom toxicity threshold for this channel (0 to use server default)
#[poise::command(slash_command, guild_only, rename = "set")]
pub async fn set_threshold(
    ctx: Context<'_>,
    #[description = "Toxicity threshold (0 for global, or 1-100)"] threshold: i64,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let channel_id = ctx.channel_id().get() as i64;
    let db = &ctx.data().db;

    sqlx::query!(
        "INSERT INTO channel_configs (channel_id, guild_id, toxicity_threshold) 
         VALUES (?, ?, ?) 
         ON CONFLICT(channel_id) DO UPDATE SET toxicity_threshold = excluded.toxicity_threshold",
        channel_id,
        guild_id,
        threshold
    ).execute(db).await?;

    {
        let mut cache = ctx.data().channel_cache.write().await;
        cache.remove(&(ctx.channel_id().get()));
    }

    let threshold_str = if threshold == 0 { "Server Default".to_string() } else { threshold.to_string() };
    ctx.say(format!("✅ Message toxicity threshold for this channel set to **{}**.", threshold_str)).await?;
    Ok(())
}

/// Toggle relaxed mode for this channel (reduces filter strictness)
#[poise::command(slash_command, guild_only, rename = "relaxed")]
pub async fn toggle_relaxed(
    ctx: Context<'_>,
    #[description = "Enable relaxed filtering?"] enabled: bool,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let channel_id = ctx.channel_id().get() as i64;
    let db = &ctx.data().db;

    sqlx::query!(
        "INSERT INTO channel_configs (channel_id, guild_id, is_relaxed) 
         VALUES (?, ?, ?) 
         ON CONFLICT(channel_id) DO UPDATE SET is_relaxed = excluded.is_relaxed",
        channel_id,
        guild_id,
        enabled
    ).execute(db).await?;

    {
        let mut cache = ctx.data().channel_cache.write().await;
        cache.remove(&(ctx.channel_id().get()));
    }

    ctx.say(format!("✅ Relaxed filtering for this channel: **{}**.", if enabled { "Enabled" } else { "Disabled" })).await?;
    Ok(())
}

/// View the current filtering status for this channel
#[poise::command(slash_command, guild_only, rename = "status")]
pub async fn view_status(ctx: Context<'_>) -> Result<(), Error> {
    let channel_id = ctx.channel_id();
    let config = ctx.data().get_channel_config(channel_id).await;
    let global_config = ctx.data().get_config(ctx.guild_id().unwrap()).await;

    let (threshold, relaxed) = match config {
        Some(c) => (c.toxicity_threshold, c.is_relaxed),
        None => (0, false),
    };

    let global_threshold = global_config.map(|c| c.toxicity_threshold).unwrap_or(0);

    let embed = serenity::CreateEmbed::new()
        .title("🛡️ Channel Filtering")
        .colour(serenity::Colour::BLURPLE)
        .field("Threshold", if threshold == 0 { format!("Server Default ({})", global_threshold) } else { threshold.to_string() }, true)
        .field("Relaxed Mode", if relaxed { "ON" } else { "OFF" }, true)
        .description(if relaxed { "💡 Filtering is less strict in this channel." } else { "⚖️ Standard security filters are active." });

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
