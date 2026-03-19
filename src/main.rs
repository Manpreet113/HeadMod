mod commands;
mod types;

use poise::serenity_prelude as serenity;
use types::*;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let token =
        std::env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN in environment / .env file");

    let guild_id =
        std::env::var("GUILD_ID").expect("Missing GuildId in environment / .env file");


    let intents = serenity::GatewayIntents::non_privileged();
    println!("Connecting to Discord...");

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::general::ping(),
                commands::moderation::kick(),
                commands::moderation::ban(),
            ],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_in_guild(
                    ctx,
                    &framework.options().commands,
                    serenity::GuildId::new(guild_id.parse()?),
                ).await?;
                println!("Head Mod is online!");
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}
