use std::sync::Arc;
use dashmap::DashMap;
use poise::serenity_prelude as serenity;

/// A single warning entry stored against a user.
#[derive(Debug, Clone)]
pub struct WarnEntry {
    pub reason:     String,
    pub moderator:  serenity::UserId,
    pub timestamp:  chrono::DateTime<chrono::Utc>,
}

/// Bot-wide shared state. Passed into every command via `Context`.
#[derive(Debug, Clone)]
pub struct Data {
    /// The single guild this bot is registered in.
    pub guild_id: serenity::GuildId,
    /// Channel where mod actions (kick, ban, unban, timeout, warn) are posted.
    pub mod_log_channel: serenity::ChannelId,
    /// Channel where deleted/edited messages are posted.
    pub message_log_channel: serenity::ChannelId,
    /// In-memory warn store: UserId → list of warnings.
    /// DashMap is used so multiple async tasks can read/write concurrently
    /// without wrapping the whole thing in a Mutex.
    pub warns: Arc<DashMap<serenity::UserId, Vec<WarnEntry>>>,

    // ── Auto-timeout config ──────────────────────────────────────────────
    /// Number of warnings before an automatic timeout is applied.
    pub warn_threshold: usize,
    /// Duration of the automatic timeout in seconds.
    pub warn_timeout_secs: i64,
}

impl Data {
    pub fn new(
        guild_id: serenity::GuildId,
        mod_log_channel: serenity::ChannelId,
        message_log_channel: serenity::ChannelId,
    ) -> Self {
        Self {
            guild_id,
            mod_log_channel,
            message_log_channel,
            warns: Arc::new(DashMap::new()),
            warn_threshold: 3,
            warn_timeout_secs: 3600, // 1 hour
        }
    }
}

/// A catch-all error type. `Box<dyn Error>` lets any error flow through.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// The handle every command receives — gives access to the message,
/// the user, the channel, `Data`, and methods to reply.
pub type Context<'a> = poise::Context<'a, Data, Error>;