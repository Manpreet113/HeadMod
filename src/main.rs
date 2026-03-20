mod commands;
mod events;
mod background_jobs;
mod logging;
mod types;

use poise::serenity_prelude as serenity;
use types::*;
use events::DataKey;

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

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .init();

    let token = std::env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN");
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data.db".to_string());
    let db = sqlx::SqlitePool::connect(&db_url).await.expect("Failed to connect to SQLite");
    
    let guild_id_opt = std::env::var("GUILD_ID").ok().and_then(|id| id.parse::<u64>().ok()).map(serenity::GuildId::new);

    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILD_MESSAGES
        | serenity::GatewayIntents::GUILD_MEMBERS;

    let db_for_setup = db.clone();
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::general::ping(),
                commands::general::help(),
                commands::moderation::kick::kick(),
                commands::moderation::ban::ban(),
                commands::moderation::unban::unban(),
                commands::moderation::timeout::timeout(),
                commands::moderation::tempban::tempban(),
                commands::moderation::warn::warn(),
                commands::moderation::purge::purge(),
                commands::moderation::logs::logs(),
                commands::moderation::case::case(),
                commands::moderation::stats::stats(),
                commands::moderation::mod_center::center(),
                commands::moderation::history::history(),
                commands::moderation::report::report_message(),
                commands::moderation::role::role(),
                commands::moderation::whois::whois(),
                commands::moderation::lockdown::lockdown(),
                commands::moderation::lockdown::unlock(),
                commands::moderation::emergency::emergency(),
                commands::moderation::utility::slowmode(),
                commands::moderation::filters::channel_filter(),
                commands::announcements::announce(),
                commands::tickets::ticket(),
                commands::config::setup(),
                commands::embeds::embed(),
            ],
            on_error: |err| Box::pin(on_error(err)),
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                if let Some(guild_id) = guild_id_opt {
                    poise::builtins::register_in_guild(ctx, &framework.options().commands, guild_id).await?;
                } else {
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                }
                let data = Data::new(db_for_setup);
                ctx.data.write().await.insert::<DataKey>(std::sync::Arc::new(data.clone()));
                Ok(data)
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .event_handler(events::MessageLogHandler)
        .event_handler(events::AutomodHandler)
        .event_handler(events::InteractionHandler)
        .event_handler(events::PersistenceHandler)
        .event_handler(events::MemberHandler)
        .await
        .expect("Failed to build Discord client");

    let mut client = client;
    client.cache.set_max_messages(1000);
    let data_arc = std::sync::Arc::new(Data::new(db.clone()));
    let http_arc = client.http.clone();
    tokio::spawn(crate::background_jobs::start_background_jobs(data_arc, http_arc));
    client.start().await.expect("Client encountered a fatal error");
}