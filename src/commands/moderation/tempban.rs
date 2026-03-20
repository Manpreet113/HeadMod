use poise::serenity_prelude as serenity;
use crate::{Context, Error};
use super::actions::{execute_ban, hierarchy_check, parse_duration, ActionResult, BanParams};

/// Temporarily ban a member. Format: /tempban @user 7d reason
#[poise::command(
    slash_command, guild_only,
    default_member_permissions = "BAN_MEMBERS",
    required_bot_permissions   = "BAN_MEMBERS"
)]
pub async fn tempban(
    ctx: Context<'_>,
    #[description = "The member to ban"] member: serenity::Member,
    #[description = "Duration (e.g. 10m, 2h, 7d)"] duration: String,
    #[description = "Reason for the ban"] reason: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;

    if member.user.id == ctx.framework().bot_id {
        ctx.say("Nice try. I'm not banning myself.").await?;
        return Ok(());
    }

    let duration_secs = match parse_duration(&duration) {
        Some(s) => s,
        None    => {
            ctx.say("❌ Invalid duration. Use a number followed by `s`, `m`, `h`, or `d` (e.g. `10m`).").await?;
            return Ok(());
        }
    };

    let reason = reason.as_deref().unwrap_or("No reason provided");

    let (invoker_can, bot_can, guild_name) = {
        let guild = ctx.guild().ok_or("Could not fetch guild")?;
        let (ic, bc) = hierarchy_check(&guild, ctx.author().id, ctx.framework().bot_id, member.user.id);
        (ic, bc, guild.name.clone())
    };

    if !invoker_can {
        ctx.say("❌ You can't ban someone at or above your own role.").await?;
        return Ok(());
    }
    if !bot_can {
        ctx.say("❌ I can't ban someone at or above my own role.").await?;
        return Ok(());
    }

    let result = execute_ban(BanParams {
        http: &ctx.http(),
        data: ctx.data(),
        invoker: ctx.author(),
        member: &member,
        guild_name: &guild_name,
        reason,
        delete_days: 0,
    }).await;

    match result {
        ActionResult::Ok(embed) => {
            let execute_at = chrono::Utc::now()
                .checked_add_signed(chrono::Duration::seconds(duration_secs))
                .unwrap_or(chrono::Utc::now());
                
            let gid = member.guild_id.get() as i64;
            let tid = member.user.id.get() as i64;

            sqlx::query!(
                "INSERT INTO scheduled_tasks (guild_id, target_id, task_type, execute_at) VALUES (?, ?, 'unban', ?)",
                gid, tid, execute_at
            ).execute(&ctx.data().db).await?;

            ctx.send(poise::CreateReply::default()
                .embed(embed.footer(serenity::CreateEmbedFooter::new(format!("Expires: {}", execute_at.format("%Y-%m-%d %H:%M:%S UTC")))))
            ).await?;
        }
        ActionResult::DiscordError(e)   => { ctx.say(format!("❌ Couldn't ban **{}**: {}", member.user.name, e)).await?; }
        ActionResult::InvalidInput(msg) => { ctx.say(format!("❌ {}", msg)).await?; }
    }

    Ok(())
}
