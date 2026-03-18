// Shared types that every part of the bot needs.

/// Bot-wide shared state. Passed into every command via `Context`.
/// We'll add fields here as the bot grows (e.g. database pool).
pub struct Data {}

/// A catch-all error type. `Box<dyn Error>` lets any error flow through.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// The handle every command receives — gives access to the message,
/// the user, the channel, `Data`, and methods to reply.
pub type Context<'a> = poise::Context<'a, Data, Error>;
