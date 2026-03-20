use poise::serenity_prelude as serenity;
use crate::{Context, Error};
use crate::commands::moderation::actions::{ActionResult, execute_unban, UnbanParams};

/// Unban a previously banned user.
#[poise::command(
    slash_command, guild_only,
    default_member_permissions = "BAN_MEMBERS",
    required_bot_permissions   = "BAN_MEMBERS",
    description_localized("en-US", "Unban a previously banned user by their ID.")
)]
pub async fn unban(
    ctx: Context<'_>,
    #[description = "The user ID to unban"] user_id: serenity::UserId,
) -> Result<(), Error> {
    ctx.defer().await?;

    let guild_id = ctx.guild_id().ok_or("Could not get guild ID")?;

    let params = UnbanParams {
        http: &ctx.http(),
        data: ctx.data(),
        invoker: ctx.author(),
        guild_id,
        user_id,
        reason: "Unbanned by moderator",
    };

    match execute_unban(params).await {
        ActionResult::Ok(embed)         => { ctx.send(poise::CreateReply::default().embed(embed)).await?; }
        ActionResult::DiscordError(e)   => { ctx.say(format!("❌ Couldn't unban <@{}>: {}", user_id, e)).await?; }
        ActionResult::InvalidInput(msg) => { ctx.say(format!("❌ {}", msg)).await?; }
    }

    Ok(())
}