use poise::serenity_prelude as serenity;
use crate::{Context, Error};

/// Instantly lock the current channel for the @everyone role.
#[poise::command(
    slash_command, 
    guild_only,
    default_member_permissions = "MANAGE_CHANNELS",
    required_bot_permissions   = "MANAGE_CHANNELS"
)]
pub async fn lockdown(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let channel_id = ctx.channel_id();
    let guild_id = ctx.guild_id().unwrap();
    let everyone_role = guild_id.get();

    let overwrite = serenity::PermissionOverwrite {
        allow: serenity::Permissions::empty(),
        deny:  serenity::Permissions::SEND_MESSAGES | serenity::Permissions::ADD_REACTIONS,
        kind:  serenity::PermissionOverwriteType::Role(serenity::RoleId::new(everyone_role)),
    };

    channel_id.create_permission(&ctx.http(), overwrite).await?;
    ctx.say("🔒 **This channel is now under lockdown.**").await?;
    Ok(())
}

/// Lift a lockdown for the current channel or the entire server.
#[poise::command(
    slash_command, 
    guild_only,
    default_member_permissions = "MANAGE_CHANNELS",
    required_bot_permissions   = "MANAGE_CHANNELS"
)]
pub async fn unlock(
    ctx: Context<'_>,
    #[description = "Unlock the entire server?"] all: Option<bool>
) -> Result<(), Error> {
    ctx.defer().await?;
    let is_all = all.unwrap_or(false);
    let guild_id = ctx.guild_id().unwrap();
    let everyone_role = guild_id.get();
    let overwrite = serenity::PermissionOverwrite {
        allow: serenity::Permissions::empty(),
        deny:  serenity::Permissions::empty(),
        kind:  serenity::PermissionOverwriteType::Role(serenity::RoleId::new(everyone_role)),
    };

    if is_all {
        if !ctx.author_member().await.map(|m| m.permissions.unwrap_or_default().contains(serenity::Permissions::ADMINISTRATOR)).unwrap_or(false) {
            ctx.say("❌ Only Administrators can unlock the entire server.").await?;
            return Ok(());
        }

        let guild = ctx.guild().unwrap().clone();
        let mut count = 0;
        for channel in guild.channels(&ctx.http()).await?.values() {
            if channel.kind == serenity::ChannelType::Text {
                let _ = channel.id.create_permission(&ctx.http(), overwrite.clone()).await;
                count += 1;
            }
        }
        ctx.say(format!("🔓 **Server-wide recovery complete.** Unlocked **{}** channels.", count)).await?;
    } else {
        ctx.channel_id().create_permission(&ctx.http(), overwrite).await?;
        ctx.say("🔓 **Lockdown has been lifted for this channel.**").await?;
    }

    Ok(())
}
