use poise::serenity_prelude as serenity;
use crate::{Context, Error};

/// TRIGGER EMERGENCY PROTOCOL: Protocol Alpha.
/// Instantly locks all channels and notifies staff.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "ADMINISTRATOR",
    required_bot_permissions = "MANAGE_CHANNELS"
)]
pub async fn emergency(ctx: Context<'_>) -> Result<(), Error> {
    let interaction_id = ctx.id();
    let reply = poise::CreateReply::default()
        .content("⚠️ **WARNING: Protocol Alpha represents a total server lockdown.**\nDo you wish to proceed?")
        .components(vec![
            serenity::CreateActionRow::Buttons(vec![
                serenity::CreateButton::new(format!("{}_confirm", interaction_id)).label("CONFIRM PROTOCOL ALPHA").style(serenity::ButtonStyle::Danger),
                serenity::CreateButton::new(format!("{}_cancel", interaction_id)).label("Cancel").style(serenity::ButtonStyle::Secondary),
            ])
        ]);

    ctx.send(reply).await?;

    let collector = serenity::ComponentInteractionCollector::new(ctx.serenity_context())
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .timeout(std::time::Duration::from_secs(30))
        .filter(move |m| m.data.custom_id.starts_with(&interaction_id.to_string()))
        .await;

    if let Some(m) = collector {
        if m.data.custom_id.ends_with("_confirm") {
            let _ = m.defer(&ctx.http()).await;
            let guild = ctx.guild().unwrap().clone();
            let mut locked_count = 0;

            // 1. Lockdown all text channels
            for channel in guild.channels(&ctx.http()).await?.values() {
                if channel.kind == serenity::ChannelType::Text {
                    let overwrite = serenity::PermissionOverwrite {
                        allow: serenity::Permissions::empty(),
                        deny: serenity::Permissions::SEND_MESSAGES,
                        kind: serenity::PermissionOverwriteType::Role(serenity::RoleId::new(ctx.guild_id().unwrap().get())),
                    };
                    let _ = channel.id.create_permission(&ctx.http(), overwrite).await;
                    locked_count += 1;
                }
            }

            // 2. Notify all moderators (using the config)
            let data = ctx.data();
            if let Some(config) = data.get_config(ctx.guild_id().unwrap()).await {
                if let Some(log_chan_id) = config.mod_log_channel_id {
                    let log_chan = serenity::ChannelId::new(log_chan_id as u64);
                    let _ = log_chan.say(&ctx.http(), "🚨 **PROTOCOL ALPHA TRIGGERED**\nAll channels have been locked. Check audit logs for details.").await;
                }
            }

            let _ = m.edit_response(&ctx.http(), serenity::EditInteractionResponse::new()
                .content(format!("✅ **PROTOCOL ALPHA EXECUTED.**\nLocked **{}** channels. Server is now in high-security state.", locked_count))
                .components(vec![])).await;
            
            // Log action
            if let Ok(bot) = ctx.http().get_current_user().await {
                crate::logging::log_mod_action(
                    &ctx.http(), data, ctx.guild_id().unwrap(), &bot.into(), &ctx.author(), 
                    crate::logging::ModAction::Timeout { reason: "PROTOCOL ALPHA (Emergency Lockdown)", duration: "Indefinite" }
                ).await;
            }

        } else {
            let _ = m.edit_response(&ctx.http(), serenity::EditInteractionResponse::new()
                .content("❌ Protocol Alpha cancelled.")
                .components(vec![])).await;
        }
    } else {
        let _ = ctx.say("⏱️ Emergency confirmation timed out.").await;
    }

    Ok(())
}
