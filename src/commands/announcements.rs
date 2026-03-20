use crate::types::{Context, Error};
use poise::serenity_prelude as serenity;

/// Create a professional announcement or custom embed
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_MESSAGES")]
pub async fn announce(
    ctx: Context<'_>,
    #[description = "Channel to post in"] channel: serenity::Channel,
    #[description = "The announcement message content"] content: String,
    #[description = "Optional embed title"] title: Option<String>,
    #[description = "Optional hex color (e.g. #FF0000)"] color: Option<String>,
) -> Result<(), Error> {
    let channel_id = channel.id();
    let mut builder = serenity::CreateMessage::new();
    
    if let Some(t) = title {
        let mut embed = serenity::CreateEmbed::new()
            .title(t)
            .description(&content);
            
        if let Some(c) = color {
            if let Ok(c_val) = u32::from_str_radix(c.trim_start_matches('#'), 16) {
                embed = embed.colour(c_val);
            }
        } else {
            embed = embed.colour(serenity::Colour::BLUE);
        }
        
        builder = builder.embed(embed);
    } else {
        builder = builder.content(&content);
    }

    match channel_id.send_message(&ctx.http(), builder).await {
        Ok(_) => ctx.say(format!("✅ Announcement sent to <#{}>.", channel_id.get())).await?,
        Err(e) => ctx.say(format!("❌ Failed to send: {}", e)).await?,
    };

    Ok(())
}
