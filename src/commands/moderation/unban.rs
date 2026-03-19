use poise::serenity_prelude as serenity;
use crate::{Context, Error};
use crate::logging::{log_mod_action, ModAction};

/// Unban a previously banned user.
#[poise::command(
    slash_command, guild_only,
    default_member_permissions = "BAN_MEMBERS",
    required_bot_permissions   = "BAN_MEMBERS"
)]
pub async fn unban(
    ctx: Context<'_>,
    #[description = "The user ID to unban"] user_id: serenity::UserId,
) -> Result<(), Error> {
    ctx.defer().await?;

    let guild_id = ctx.guild_id().ok_or("Could not get guild ID")?;
    let bans     = guild_id.bans(&ctx.http(), None, None).await?;

    if !bans.iter().any(|b| b.user.id == user_id) {
        ctx.say(format!("❌ <@{}> is not currently banned.", user_id)).await?;
        return Ok(());
    }

    let target = user_id.to_user(&ctx.http()).await?;

    match guild_id.unban(&ctx.http(), user_id).await {
        Ok(()) => {
            log_mod_action(&ctx.http(), ctx.data(), ctx.author(), &target,
                ModAction::Unban).await;
            ctx.say(format!("✅ <@{}> has been unbanned.", user_id)).await?;
        }
        Err(e) => {
            ctx.say(format!("❌ Couldn't unban <@{}>: {}", user_id, e)).await?;
        }
    }

    Ok(())
}