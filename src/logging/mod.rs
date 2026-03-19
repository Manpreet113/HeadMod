use poise::serenity_prelude as serenity;
use crate::types::Data;

/// The kind of moderation action being logged.
pub enum ModAction<'a> {
    Kick    { reason: &'a str },
    Ban     { reason: &'a str, delete_days: u8 },
    Unban,
    Timeout { reason: &'a str, duration: &'a str },
    Warn    { reason: &'a str, warn_count: usize, auto_timeout: bool },
}

/// Post a mod action embed to the mod log channel.
/// Failures are logged with `tracing::error!` and swallowed.
pub async fn log_mod_action(
    http:      &serenity::Http,
    data:      &Data,
    moderator: &serenity::User,
    target:    &serenity::User,
    action:    ModAction<'_>,
) {
    let (title, colour, extra) = match action {
        ModAction::Kick { reason } => (
            "👢 Member Kicked", serenity::Colour::ORANGE,
            format!("**Reason:** {}", reason),
        ),
        ModAction::Ban { reason, delete_days } => (
            "🔨 Member Banned", serenity::Colour::RED,
            format!("**Reason:** {}\n**Messages deleted:** {} day(s)", reason, delete_days),
        ),
        ModAction::Unban => (
            "✅ Member Unbanned", serenity::Colour::DARK_GREEN,
            String::new(),
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

    let embed = serenity::CreateEmbed::new()
        .title(title)
        .colour(colour)
        .field("Target",    format!("{} (`{}`)", target.name,    target.id),    false)
        .field("Moderator", format!("{} (`{}`)", moderator.name, moderator.id), false)
        .description(extra)
        .timestamp(serenity::Timestamp::now());

    let msg = serenity::CreateMessage::new().embed(embed);
    if let Err(e) = data.mod_log_channel.send_message(http, msg).await {
        tracing::error!(channel = %data.mod_log_channel, error = %e, "Failed to send mod log");
    }
}