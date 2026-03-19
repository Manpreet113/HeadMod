// Shared types that every part of the bot needs.

use poise::serenity_prelude as serenity;
/// Bot-wide shared state. Passed into every command via `Context`.
/// We'll add fields here as the bot grows (e.g. database pool).

#[derive(Debug, Clone)]
pub struct Data {
    pub guild_id: serenity::GuildId,
    pub mod_log_channel: serenity::ChannelId,
    pub message_log_channel: serenity::ChannelId,
}

/// A catch-all error type. `Box<dyn Error>` lets any error flow through.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// The handle every command receives — gives access to the message,
/// the user, the channel, `Data`, and methods to reply.
pub type Context<'a> = poise::Context<'a, Data, Error>;
