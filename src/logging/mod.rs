use poise::serenity_prelude as serenity;
use crate::types::Data;

pub enum ModAction<'a> {
    Kick    { reason: &'a str },
    Ban     { reason: &'a str, delete_days: u8 },
    Unban   { reason: &'a str },
    Timeout { reason: &'a str, duration: &'a str },
    Warn    { reason: &'a str, warn_count: usize, auto_timeout: bool },
}

pub async fn log_mod_action(
    http:      &serenity::Http,
    data:      &Data,
    guild_id:  serenity::GuildId,
    moderator: &serenity::User,
    target:    &serenity::User,
    action:    ModAction<'_>,
) -> u64 {
    let (title, colour, extra) = match action {
        ModAction::Kick { reason } => (
            "👢 Member Kicked", serenity::Colour::ORANGE,
            format!("**Reason:** {}", reason),
        ),
        ModAction::Ban { reason, delete_days } => (
            "🔨 Member Banned", serenity::Colour::RED,
            format!("**Reason:** {}\n**Messages deleted:** {} day(s)", reason, delete_days),
        ),
        ModAction::Unban { reason } => (
            "✅ Member Unbanned", serenity::Colour::DARK_GREEN,
            format!("**Reason:** {}", reason),
        ),
        ModAction::Timeout { reason, duration } => (
            "⏱️ Member Timed Out", serenity::Colour::GOLD,
            format!("**Duration:** {}\n**Reason:** {}", duration, reason),
        ),
        ModAction::Warn { reason, warn_count, auto_timeout } => (
            "⚠️ Member Warned", serenity::Colour::ORANGE,
            format!(
                "**Reason:** {}\n**Total warnings:** {}\n{}",
                reason, warn_count,
                if auto_timeout { "⏱️ Auto-timeout applied." } else { "" },
            ),
        ),
    };

    let action_str = match action {
        ModAction::Kick { .. } => "kick",
        ModAction::Ban { .. } => "ban",
        ModAction::Unban { .. } => "unban",
        ModAction::Timeout { .. } => "timeout",
        ModAction::Warn { .. } => "warn",
    };
    
    let reason = match action {
        ModAction::Kick { reason } => reason,
        ModAction::Ban { reason, .. } => reason,
        ModAction::Unban { reason } => reason,
        ModAction::Timeout { reason, .. } => reason,
        ModAction::Warn { reason, .. } => reason,
    };
    
    let duration_secs = match action {
        ModAction::Timeout { duration, .. } => crate::commands::moderation::actions::parse_duration(duration),
        _ => None,
    };
    
    let gid = guild_id.get() as i64;
    let tid = target.id.get() as i64;
    let mid = moderator.id.get() as i64;

    let case_id = sqlx::query!(
        "INSERT INTO cases (guild_id, target_id, moderator_id, action_type, reason, duration_secs) VALUES (?, ?, ?, ?, ?, ?)",
        gid, tid, mid, action_str, reason, duration_secs
    ).execute(&data.db).await.map(|r| r.last_insert_rowid() as u64).unwrap_or(0);

    let embed = serenity::CreateEmbed::new()
        .title(title)
        .colour(colour)
        .field("Target",    format!("{} (`{}`)", target.name,    target.id),    false)
        .field("Moderator", format!("{} (`{}`)", moderator.name, moderator.id), false)
        .description(extra)
        .footer(serenity::CreateEmbedFooter::new(format!("Case #{}", case_id)))
        .timestamp(serenity::Timestamp::now());

    let msg = serenity::CreateMessage::new().embed(embed);
    if let Ok(Some(row)) = sqlx::query!("SELECT mod_log_channel_id FROM guild_configs WHERE guild_id = ?", gid).fetch_optional(&data.db).await {
        if let Some(log_id) = row.mod_log_channel_id {
            let chan = serenity::ChannelId::new(log_id as u64);
            if let Err(e) = chan.send_message(http, msg).await {
                tracing::error!(channel = %chan, error = %e, "Failed to send mod log");
            }
        }
    }

    case_id
}