mod commands;
mod events;
mod logging;
mod types;

use poise::serenity_prelude as serenity;
use types::*;
use events::{MessageLogHandler, DataKey};

/// Central error handler — called by Poise whenever a command returns Err
/// or a pre-/post-command hook fails.
async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            tracing::error!("Error in command '{}': {:?}", ctx.command().name, error);
            let _ = ctx.say("⚠️ Something went wrong running that command.").await;
        }
        poise::FrameworkError::MissingUserPermissions { missing_permissions, ctx, .. } => {
            let msg = match missing_permissions {
                Some(p) => format!("❌ You're missing permissions: `{}`", p),
                None    => "❌ You don't have permission to use this command.".to_owned(),
            };
            let _ = ctx.say(msg).await;
        }
        poise::FrameworkError::MissingBotPermissions { missing_permissions, ctx, .. } => {
            let _ = ctx.say(format!(
                "❌ I'm missing permissions to do that: `{}`", missing_permissions
            )).await;
        }
        other => {
            if let Err(e) = poise::builtins::on_error(other).await {
                tracing::error!("Unhandled framework error: {:?}", e);
            }
        }
    }
}

/// Parse a required `u64` env var, panicking with a clear message on failure.
fn parse_id(var: &str) -> u64 {
    std::env::var(var)
        .unwrap_or_else(|_| panic!("Missing {} in environment / .env file", var))
        .parse()
        .unwrap_or_else(|_| panic!("{} must be a valid integer", var))
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    // Initialise tracing. RUST_LOG controls the filter at runtime,
    // e.g. `RUST_LOG=head_mod=debug,warn cargo run`.
    // Falls back to INFO for everything if RUST_LOG is not set.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let token           = std::env::var("DISCORD_TOKEN")
        .expect("Missing DISCORD_TOKEN in environment / .env file");
    let guild_id        = serenity::GuildId::new(parse_id("GUILD_ID"));
    let mod_log_channel = serenity::ChannelId::new(parse_id("MOD_LOG_CHANNEL_ID"));
    let msg_log_channel = serenity::ChannelId::new(parse_id("MESSAGE_LOG_CHANNEL_ID"));

    // MESSAGE_CONTENT lets us read message text for the edit/delete logs.
    // Make sure this privileged intent is enabled in the Developer Portal.
    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILD_MESSAGES;

    tracing::info!("Connecting to Discord...");

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::general::ping(),
                commands::general::help(),
                commands::moderation::kick(),
                commands::moderation::ban(),
                commands::moderation::unban(),
                commands::moderation::timeout(),
                commands::moderation::warn(),
                commands::moderation::purge(),
            ],
            on_error: |err| Box::pin(on_error(err)),
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_in_guild(
                    ctx,
                    &framework.options().commands,
                    guild_id,
                ).await?;

                let data = Data::new(guild_id, mod_log_channel, msg_log_channel);

                // Share a clone into serenity's TypeMap so LogHandler can
                // reach it. Poise gets its own copy via Ok(data) below.
                ctx.data.write().await.insert::<DataKey>(std::sync::Arc::new(data.clone()));

                tracing::info!("Head Mod is online!");
                Ok(data)
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .event_handler(MessageLogHandler)
        .await
        .expect("Failed to build Discord client");

    // Enable message caching so edit/delete logs can recover message content.
    // Must be called after the client is built, not during construction.
    client.cache.set_max_messages(1000);

    client.start().await.expect("Client encountered a fatal error");
}