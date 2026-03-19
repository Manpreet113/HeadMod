use chrono::Utc;
use poise::serenity_prelude as serenity;
use crate::{Context, Error};
use crate::types::WarnEntry;
use crate::logging::{log_mod_action, ModAction};

// ── Duration parsing ──────────────────────────────────────────────────────────

/// Parse a human duration string like `10m`, `2h`, `7d` into seconds.
/// Supported units: `s` seconds, `m` minutes, `h` hours, `d` days.
fn parse_duration(s: &str) -> Option<i64> {
    let s = s.trim();
    let (num_part, unit) = s.split_at(s.len().checked_sub(1)?);
    let n: i64 = num_part.parse().ok()?;
    if n <= 0 { return None; }
    match unit {
        "s" => Some(n),
        "m" => Some(n * 60),
        "h" => Some(n * 3600),
        "d" => Some(n * 86_400),
        _   => None,
    }
}

// ── /timeout ──────────────────────────────────────────────────────────────────

/// Temporarily mute a member. Duration format: 10s, 5m, 2h, 7d (max 28d).
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MODERATE_MEMBERS",
    required_bot_permissions   = "MODERATE_MEMBERS"
)]
pub async fn timeout(
    ctx: Context<'_>,
    #[description = "Member to timeout"] member: serenity::Member,
    #[description = "Duration (e.g. 10m, 2h, 7d — max 28d)"] duration: String,
    #[description = "Reason"] reason: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;

    let reason = reason.as_deref().unwrap_or("No reason provided");

    let secs = match parse_duration(&duration) {
        Some(s) => s,
        None => {
            ctx.say("❌ Invalid duration. Use a number followed by `s`, `m`, `h`, or `d` (e.g. `10m`, `2h`).").await?;
            return Ok(());
        }
    };

    const MAX_SECS: i64 = 28 * 24 * 3600;
    if secs > MAX_SECS {
        ctx.say("❌ Maximum timeout duration is 28 days.").await?;
        return Ok(());
    }

    if member.user.id == ctx.framework().bot_id {
        ctx.say("Nice try. I'm not timing myself out.").await?;
        return Ok(());
    }

    let until = Utc::now()
        .checked_add_signed(chrono::Duration::seconds(secs))
        .ok_or("Duration overflow")?;

    let timestamp = serenity::Timestamp::from_unix_timestamp(until.timestamp())?;

    // disable_communication_until takes an ISO 8601 String, not a Timestamp.
    let edit = serenity::EditMember::new()
        .disable_communication_until(timestamp.to_string());

    match member.guild_id.edit_member(&ctx.http(), member.user.id, edit).await {
        Ok(_) => {
            ctx.say(format!(
                "⏱️ **{}** has been timed out for **{}**.\n**Reason:** {}",
                member.user.name, duration, reason,
            )).await?;
            log_mod_action(
                &ctx.http(),
                ctx.data(),
                ctx.author(),
                &member.user,
                ModAction::Timeout { reason, duration: &duration },
            ).await;
        }
        Err(e) => {
            ctx.say(format!("❌ Couldn't timeout **{}**: {}", member.user.name, e)).await?;
        }
    }

    Ok(())
}

// ── /warn ─────────────────────────────────────────────────────────────────────

/// Warn a member. At the threshold, an automatic timeout is applied.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MODERATE_MEMBERS",
    required_bot_permissions   = "MODERATE_MEMBERS",
    subcommands("add", "list", "clear")
)]
pub async fn warn(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add a warning to a member.
#[poise::command(slash_command, guild_only)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "Member to warn"] member: serenity::Member,
    #[description = "Reason for the warning"] reason: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let data = ctx.data();

    let entry = WarnEntry {
        reason:    reason.clone(),
        moderator: ctx.author().id,
        timestamp: Utc::now(),
    };

    let warn_count = {
        let mut warns = data.warns.entry(member.user.id).or_default();
        warns.push(entry);
        warns.len()
    };

    let mut auto_timeout = false;

    if warn_count >= data.warn_threshold {
        let until = Utc::now()
            .checked_add_signed(chrono::Duration::seconds(data.warn_timeout_secs))
            .ok_or("Duration overflow")?;
        let timestamp = serenity::Timestamp::from_unix_timestamp(until.timestamp())?;
        let edit = serenity::EditMember::new()
            .disable_communication_until(timestamp.to_string());

        match member.guild_id.edit_member(&ctx.http(), member.user.id, edit).await {
            Ok(_) => {
                auto_timeout = true;
                tracing::info!(
                    user = %member.user.id,
                    warns = warn_count,
                    "Auto-timeout applied after warn threshold reached"
                );
            }
            Err(e) => {
                tracing::error!("Failed to apply auto-timeout for {}: {}", member.user.id, e);
            }
        }
    }

    let dm_content = if auto_timeout {
        format!(
            "You have been warned in this server.\n**Reason:** {}\n**Total warnings:** {}\nYou have also been automatically timed out.",
            reason, warn_count,
        )
    } else {
        format!(
            "You have been warned in this server.\n**Reason:** {}\n**Total warnings:** {}",
            reason, warn_count,
        )
    };
    let _ = member.user.dm(&ctx.http(), serenity::CreateMessage::new().content(dm_content)).await;

    ctx.say(format!(
        "⚠️ **{}** has been warned. Total warnings: **{}**{}",
        member.user.name,
        warn_count,
        if auto_timeout { "\n⏱️ Auto-timeout applied." } else { "" },
    )).await?;

    log_mod_action(
        &ctx.http(),
        ctx.data(),
        ctx.author(),
        &member.user,
        ModAction::Warn { reason: &reason, warn_count, auto_timeout },
    ).await;

    Ok(())
}

/// List all warnings for a member.
#[poise::command(slash_command, guild_only)]
pub async fn list(
    ctx: Context<'_>,
    #[description = "Member to check"] member: serenity::Member,
) -> Result<(), Error> {
    ctx.defer().await?;

    let data = ctx.data();
    let warns = data.warns.get(&member.user.id);
    let warns = warns.as_deref();

    if warns.map(|w| w.is_empty()).unwrap_or(true) {
        ctx.say(format!("✅ **{}** has no warnings.", member.user.name)).await?;
    } else {
        let warns = warns.unwrap();
        let lines: Vec<String> = warns
            .iter()
            .enumerate()
            .map(|(i, w)| format!(
                "**{}**. {} — by <@{}> on <t:{}:D>",
                i + 1,
                w.reason,
                w.moderator,
                w.timestamp.timestamp(),
            ))
            .collect();

        ctx.say(format!(
            "⚠️ **{}** has **{}** warning(s):\n{}",
            member.user.name,
            warns.len(),
            lines.join("\n"),
        )).await?;
    }

    Ok(())
}

/// Clear all warnings for a member.
#[poise::command(slash_command, guild_only)]
pub async fn clear(
    ctx: Context<'_>,
    #[description = "Member to clear warnings for"] member: serenity::Member,
) -> Result<(), Error> {
    ctx.defer().await?;

    ctx.data().warns.remove(&member.user.id);
    ctx.say(format!("✅ Cleared all warnings for **{}**.", member.user.name)).await?;

    Ok(())
}

// ── /purge ────────────────────────────────────────────────────────────────────

/// Bulk-delete messages in this channel (max 100, must be under 14 days old).
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MANAGE_MESSAGES",
    required_bot_permissions   = "MANAGE_MESSAGES"
)]
pub async fn purge(
    ctx: Context<'_>,
    #[description = "Number of messages to delete (1–100)"] amount: u8,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let amount = amount.clamp(1, 100);
    let channel_id = ctx.channel_id();

    let messages = channel_id
        .messages(&ctx.http(), serenity::GetMessages::new().limit(amount + 1))
        .await?;

    let cutoff = Utc::now() - chrono::Duration::days(14);
    let ids: Vec<serenity::MessageId> = messages
        .iter()
        .filter(|m| m.timestamp.unix_timestamp() > cutoff.timestamp())
        .map(|m| m.id)
        .take(amount as usize)
        .collect();

    if ids.is_empty() {
        ctx.say("❌ No messages found that can be bulk-deleted (all are older than 14 days).").await?;
        return Ok(());
    }

    let deleted = ids.len();
    channel_id.delete_messages(&ctx.http(), &ids).await?;

    ctx.say(format!("🗑️ Deleted **{}** message(s).", deleted)).await?;

    Ok(())
}