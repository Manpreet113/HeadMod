use poise::serenity_prelude as serenity;
use crate::{Context, Error};
use crate::logging::{log_mod_action, ModAction};

// ── helpers ──────────────────────────────────────────────────────────────────

/// Checks role hierarchy and returns a pair of `bool`s:
/// `(invoker_can_act, bot_can_act)`.
///
/// Extracts everything it needs from the cache into plain owned values so the
/// `CacheRef` guard is dropped before we ever hit an `.await`. This is the key
/// requirement for keeping command futures `Send`.
///
/// Discord rule: you can only act on someone whose highest role position is
/// strictly below your own. The guild owner can always act; nobody can act on
/// the owner.
fn hierarchy_check(
    guild: &serenity::Guild,
    invoker_id: serenity::UserId,
    bot_id: serenity::UserId,
    target_id: serenity::UserId,
) -> (bool, bool) {
    // Nobody can act on the guild owner.
    if target_id == guild.owner_id {
        return (false, false);
    }

    let top_position = |user_id: serenity::UserId| -> u16 {
        guild
            .members
            .get(&user_id)
            .and_then(|m| guild.member_highest_role(m))
            .map(|r| r.position)
            .unwrap_or(0)
    };

    let target_pos   = top_position(target_id);
    let invoker_can  = invoker_id == guild.owner_id || top_position(invoker_id) > target_pos;
    let bot_can      = top_position(bot_id) > target_pos;

    (invoker_can, bot_can)
}

// ── commands ─────────────────────────────────────────────────────────────────

/// Kick a member from the server.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "KICK_MEMBERS",
    required_bot_permissions   = "KICK_MEMBERS"
)]
pub async fn kick(
    ctx: Context<'_>,
    #[description = "The member to kick"] member: serenity::Member,
    #[description = "Reason for the kick"] reason: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;

    let reason = reason.as_deref().unwrap_or("No reason provided");

    // Self-kick guard.
    if member.user.id == ctx.framework().bot_id {
        ctx.say("Nice try. I'm not kicking myself.").await?;
        return Ok(());
    }

    // Extract hierarchy result into plain bools, then drop the CacheRef
    // *completely* before the first .await. If the CacheRef crosses an await
    // point the future becomes non-Send and won't compile.
    let (invoker_can_act, bot_can_act) = {
        let guild = ctx.guild().ok_or("Could not fetch guild")?;
        hierarchy_check(&guild, ctx.author().id, ctx.framework().bot_id, member.user.id)
        // CacheRef dropped here — only plain bools remain.
    };

    if !invoker_can_act {
        ctx.say("❌ You can't kick someone at or above your own role.").await?;
        return Ok(());
    }
    if !bot_can_act {
        ctx.say("❌ I can't kick someone at or above my own role.").await?;
        return Ok(());
    }

    // Best-effort DM — we don't bail if it fails (user may have DMs off).
    let dm = serenity::CreateMessage::new().content(format!(
        "You have been kicked from **<{}>**.\nReason: {}",
        ctx.guild().unwrap().name,
        reason,
    ));
    let _ = member.user.dm(&ctx.http(), dm).await;

    match member.kick_with_reason(&ctx.http(), reason).await {
        Ok(()) => {
            ctx.say(format!(
                "👢 **{}** has been kicked.\n**Reason:** {}",
                member.user.name, reason,
            )).await?;
            log_mod_action(
                &ctx.http(),
                ctx.data(),
                ctx.author(),
                &member.user,
                ModAction::Kick { reason },
            ).await;
        }
        Err(e) => {
            ctx.say(format!(
                "❌ Couldn't kick **{}**: {}",
                member.user.name, e,
            )).await?;
        }
    }

    Ok(())
}

/// Permanently ban a member from the server.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "BAN_MEMBERS",
    required_bot_permissions   = "BAN_MEMBERS"
)]
pub async fn ban(
    ctx: Context<'_>,
    #[description = "The user to ban"] member: serenity::Member,
    #[description = "Reason for the ban"] reason: Option<String>,
    #[description = "Days of messages to delete (0–7)"] delete: Option<u8>,
) -> Result<(), Error> {
    ctx.defer().await?;

    let reason = reason.as_deref().unwrap_or("No reason provided");
    // Clamp to the range Discord actually accepts so we never forward a
    // bad value and confuse the user with a raw API error.
    let delete_days = delete.unwrap_or(0).min(7);

    if member.user.id == ctx.framework().bot_id {
        ctx.say("Nice try. I'm not banning myself.").await?;
        return Ok(());
    }

    let (invoker_can_act, bot_can_act) = {
        let guild = ctx.guild().ok_or("Could not fetch guild")?;
        hierarchy_check(&guild, ctx.author().id, ctx.framework().bot_id, member.user.id)
    };

    if !invoker_can_act {
        ctx.say("❌ You can't ban someone at or above your own role.").await?;
        return Ok(());
    }
    if !bot_can_act {
        ctx.say("❌ I can't ban someone at or above my own role.").await?;
        return Ok(());
    }

    let dm = serenity::CreateMessage::new().content(format!(
        "You have been banned from **{}**.\nReason: {}",
        ctx.guild().unwrap().name,
        reason,
    ));
    let _ = member.user.dm(&ctx.http(), dm).await;

    match member.ban_with_reason(&ctx.http(), delete_days, reason).await {
        Ok(()) => {
            ctx.say(format!(
                "🔨 **{}** has been banned.\n**Reason:** {}",
                member.user.name, reason,
            )).await?;
            log_mod_action(
                &ctx.http(),
                ctx.data(),
                ctx.author(),
                &member.user,
                ModAction::Ban { reason, delete_days },
            ).await;
        }
        Err(e) => {
            ctx.say(format!(
                "❌ Couldn't ban **{}**: {}",
                member.user.name, e,
            )).await?;
        }
    }

    Ok(())
}

/// Unban a previously banned user.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "BAN_MEMBERS",
    required_bot_permissions   = "BAN_MEMBERS"
)]
pub async fn unban(
    ctx: Context<'_>,
    #[description = "The user ID to unban"] user_id: serenity::UserId,
) -> Result<(), Error> {
    ctx.defer().await?;

    // Use guild_id() — we only need the ID, not the full Guild struct.
    let guild_id = ctx.guild_id().ok_or("Could not get guild ID")?;
    let bans = guild_id.bans(&ctx.http(), None, None).await?;

    if !bans.iter().any(|b| b.user.id == user_id) {
        ctx.say(format!("❌ <@{}> is not currently banned.", user_id)).await?;
        return Ok(());
    }

    let target_user = serenity::UserId::new(user_id.get()).to_user(&ctx.http()).await?;

    match guild_id.unban(&ctx.http(), user_id).await {
        Ok(()) => {
            ctx.say(format!("✅ <@{}> has been unbanned.", user_id)).await?;
            log_mod_action(
                &ctx.http(),
                ctx.data(),
                ctx.author(),
                &target_user,
                ModAction::Unban,
            ).await;
        }
        Err(e) => {
            ctx.say(format!("❌ Couldn't unban <@{}>: {}", user_id, e)).await?;
        }
    }

    Ok(())
}