use poise::serenity_prelude as serenity;
use crate::{Context, Error};

/// The ultimate configuration hub for Head Mod.
#[poise::command(
    slash_command, guild_only,
    default_member_permissions = "ADMINISTRATOR",
    required_bot_permissions   = "ADMINISTRATOR",
    description_localized("en-US", "Access the Head Mod unified setup wizard.")
)]
pub async fn setup(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let gid = ctx.guild_id().unwrap();
    let gid_i64 = gid.get() as i64;
    let interaction_id = ctx.id();
    
    let mut state = SetupState::Main;

    while !matches!(state, SetupState::Done) {
        let (embed, components, attachment) = match state {
            SetupState::Main => (
                system_embed("Head Mod Setup", "Welcome to the unified command center. Configuration is now streamlined.\n\n📊 **Status**: View active settings.\n📜 **Logging**: Audit logs & archives.\n🛡️ **Security**: Automod & protection.\n🎟️ **Community**: Tickets & verification.\n🛠️ **Advanced**: Misc & cleanup.")
                    .thumbnail("attachment://temp.png"),
                vec![
                    serenity::CreateActionRow::Buttons(vec![
                        serenity::CreateButton::new(format!("{}_status", interaction_id)).label("Status").style(serenity::ButtonStyle::Secondary).emoji('📊'),
                        serenity::CreateButton::new(format!("{}_logs", interaction_id)).label("Logging").style(serenity::ButtonStyle::Primary).emoji('📜'),
                        serenity::CreateButton::new(format!("{}_security", interaction_id)).label("Security").style(serenity::ButtonStyle::Primary).emoji(serenity::ReactionType::Unicode("🛡️".to_string())),
                    ]),
                    serenity::CreateActionRow::Buttons(vec![
                        serenity::CreateButton::new(format!("{}_community", interaction_id)).label("Community").style(serenity::ButtonStyle::Primary).emoji(serenity::ReactionType::Unicode("🎟️".to_string())),
                        serenity::CreateButton::new(format!("{}_advanced", interaction_id)).label("Advanced").style(serenity::ButtonStyle::Primary).emoji(serenity::ReactionType::Unicode("🛠️".to_string())),
                        serenity::CreateButton::new(format!("{}_done", interaction_id)).label("Close").style(serenity::ButtonStyle::Danger).emoji('✅'),
                    ])
                ],
                Some(serenity::CreateAttachment::path("temp.png").await?)
            ),
            SetupState::Status => {
                let conf = ctx.data().get_config(gid).await.unwrap_or_default();
                let desc = format!(
                    "### 📊 Active Configuration\n\
                    **Logging**\n• Mod Log: {}\n• Msg Log: {}\n• Join/Leave Logs: {}\n\n\
                    **Security**\n• Toxicity: **{}**\n• Anti-Invite: {}\n• Anti-Spam: {}\n• Global Intel: {}\n\n\
                    **Community**\n• Verification: {}\n• Tickets: {}",
                    conf.mod_log_channel_id.map(|id| format!("<#{}>", id)).unwrap_or("❌".into()),
                    conf.message_log_channel_id.map(|id| format!("<#{}>", id)).unwrap_or("❌".into()),
                    if conf.join_log_channel_id.is_some() || conf.leave_log_channel_id.is_some() { "✅ Enabled" } else { "❌ Disabled" },
                    conf.toxicity_threshold,
                    if conf.anti_invite { "✅" } else { "❌" },
                    if conf.anti_spam { "✅" } else { "❌" },
                    if conf.global_intel_enabled { "✅" } else { "❌" },
                    conf.verification_channel_id.map(|id| format!("<#{}>", id)).unwrap_or("❌".into()),
                    conf.ticket_channel_id.map(|id| format!("<#{}>", id)).unwrap_or("❌".into()),
                );
                (
                    system_embed("System Status", &desc),
                    vec![serenity::CreateActionRow::Buttons(vec![
                        serenity::CreateButton::new(format!("{}_back", interaction_id)).label("Back").style(serenity::ButtonStyle::Secondary),
                    ])],
                    None
                )
            },
            SetupState::Logs => (
                system_embed("Logging & Archives", "Keep track of every action in your server.\n\n📜 **Audit Channels**: Set where logs are sent.\n🔄 **Toggle Events**: Enable/Disable specific trackers.")
                    .image("attachment://temp2.png"),
                vec![
                    serenity::CreateActionRow::Buttons(vec![
                        serenity::CreateButton::new(format!("{}_toggle_joinleave", interaction_id)).label("Toggle Join/Leave").style(serenity::ButtonStyle::Primary),
                        serenity::CreateButton::new(format!("{}_back", interaction_id)).label("Back").style(serenity::ButtonStyle::Secondary),
                    ])
                ],
                Some(serenity::CreateAttachment::path("temp2.png").await?)
            ),
            SetupState::Security => (
                system_embed("Security Hub", "Fortify your server against raiders and bad actors.\n\n⚠️ **Toxicity**: Global threshold.\n🛡️ **Global Intel**: Cross-server history check.\n🚫 **Spam/Invites**: automated message filtering.")
                    .image("attachment://temp2.png"),
                vec![
                    serenity::CreateActionRow::Buttons(vec![
                        serenity::CreateButton::new(format!("{}_toggle_intel", interaction_id)).label("Global Intel").style(serenity::ButtonStyle::Primary).emoji(serenity::ReactionType::Unicode("🛡️".to_string())),
                        serenity::CreateButton::new(format!("{}_toggle_invite", interaction_id)).label("Anti-Invite").style(serenity::ButtonStyle::Primary),
                        serenity::CreateButton::new(format!("{}_toggle_spam", interaction_id)).label("Anti-Spam").style(serenity::ButtonStyle::Primary),
                    ]),
                    serenity::CreateActionRow::Buttons(vec![
                        serenity::CreateButton::new(format!("{}_back", interaction_id)).label("Back").style(serenity::ButtonStyle::Secondary),
                    ])
                ],
                Some(serenity::CreateAttachment::path("temp2.png").await?)
            ),
            SetupState::Community => (
                system_embed("Community Pulse", "Configure how members interact with your server.\n\n🛡️ **Verification**: Secure your gates.\n🎟️ **Tickets**: Handle support queries.")
                    .image("attachment://temp2.png"),
                vec![
                    serenity::CreateActionRow::Buttons(vec![
                        serenity::CreateButton::new(format!("{}_verify_post", interaction_id)).label("Post Verification").style(serenity::ButtonStyle::Success).emoji(serenity::ReactionType::Unicode("🛡️".to_string())),
                        serenity::CreateButton::new(format!("{}_back", interaction_id)).label("Back").style(serenity::ButtonStyle::Secondary),
                    ])
                ],
                Some(serenity::CreateAttachment::path("temp2.png").await?)
            ),
            SetupState::Advanced => (system_embed("Advanced Tools", "Miscellaneous settings and database maintenance.\n\n🧹 **Cleanup**: Purge stale data.\n⚙️ **Reset**: Standardize configuration.")
                  .image("attachment://temp2.png"),
                  vec![serenity::CreateActionRow::Buttons(vec![serenity::CreateButton::new(format!("{}_back", interaction_id)).label("Back").style(serenity::ButtonStyle::Secondary)])], 
                  Some(serenity::CreateAttachment::path("temp2.png").await?)),
            SetupState::Done => break,
        };

        let mut response = poise::CreateReply::default()
            .embed(embed)
            .components(components);
            
        if let Some(att) = attachment {
            response = response.attachment(att);
        }
            
        ctx.send(response).await?;

        let mci = serenity::ComponentInteractionCollector::new(ctx.serenity_context())
            .author_id(ctx.author().id)
            .timeout(std::time::Duration::from_secs(300))
            .filter(move |i| i.data.custom_id.starts_with(&interaction_id.to_string()))
            .await;

        match mci {
            Some(mci) => {
                mci.defer(ctx.http()).await?;
                let custom_id = mci.data.custom_id.replace(&format!("{}_", interaction_id), "");
                match custom_id.as_str() {
                    "status" => state = SetupState::Status,
                    "logs" => state = SetupState::Logs,
                    "security" => state = SetupState::Security,
                    "community" => state = SetupState::Community,
                    "advanced" => state = SetupState::Advanced,
                    "toggle_intel" => {
                        let conf = ctx.data().get_config(gid).await.unwrap_or_default();
                        let enabled = !conf.global_intel_enabled;
                        let _ = sqlx::query!("UPDATE guild_configs SET global_intel_enabled = ? WHERE guild_id = ?", enabled, gid_i64).execute(&ctx.data().db).await;
                        ctx.data().config_cache.write().await.remove(&(gid.get()));
                    },
                    "toggle_joinleave" => {
                        let conf = ctx.data().get_config(gid).await.unwrap_or_default();
                        let enabled = conf.join_log_channel_id.is_none();
                        if enabled {
                            let chan = ctx.channel_id().get() as i64;
                            let _ = sqlx::query!("UPDATE guild_configs SET join_log_channel_id = ?, leave_log_channel_id = ? WHERE guild_id = ?", chan, chan, gid_i64).execute(&ctx.data().db).await;
                        } else {
                            let _ = sqlx::query!("UPDATE guild_configs SET join_log_channel_id = NULL, leave_log_channel_id = NULL WHERE guild_id = ?", gid_i64).execute(&ctx.data().db).await;
                        }
                        ctx.data().config_cache.write().await.remove(&(gid.get()));
                    },
                    "toggle_invite" => {
                        let conf = ctx.data().get_config(gid).await.unwrap_or_default();
                        let enabled = !conf.anti_invite;
                        let _ = sqlx::query!("UPDATE guild_configs SET anti_invite = ? WHERE guild_id = ?", enabled, gid_i64).execute(&ctx.data().db).await;
                        ctx.data().config_cache.write().await.remove(&(gid.get()));
                    },
                    "toggle_spam" => {
                        let conf = ctx.data().get_config(gid).await.unwrap_or_default();
                        let enabled = !conf.anti_spam;
                        let _ = sqlx::query!("UPDATE guild_configs SET anti_spam = ? WHERE guild_id = ?", enabled, gid_i64).execute(&ctx.data().db).await;
                        ctx.data().config_cache.write().await.remove(&(gid.get()));
                    },
                    "verify_post" => {
                        let conf = ctx.data().get_config(gid).await.unwrap_or_default();
                        let v_chan_id = conf.verification_channel_id.unwrap_or(ctx.channel_id().get() as i64);
                        let channel = serenity::ChannelId::new(v_chan_id as u64);
                        let v_embed = serenity::CreateEmbed::new()
                            .title("🔐 Server Verification")
                            .description("Welcome to the server! To ensure a safe environment, please click the button below to gain access.\n\nBy verifying, you agree to follow the server rules and guidelines.")
                            .colour(serenity::Colour::from_rgb(47, 49, 54))
                            .thumbnail("attachment://temp.png")
                            .image("attachment://temp2.png")
                            .footer(serenity::CreateEmbedFooter::new("Head Mod Security Protocol"));
                        let v_components = vec![serenity::CreateActionRow::Buttons(vec![
                            serenity::CreateButton::new("verify_member").label("Verify Me").style(serenity::ButtonStyle::Success).emoji('✅')
                        ])];
                        let _ = channel.send_message(&ctx.http(), serenity::CreateMessage::new()
                            .embed(v_embed)
                            .components(v_components)
                            .add_file(serenity::CreateAttachment::path("temp.png").await?)
                            .add_file(serenity::CreateAttachment::path("temp2.png").await?)
                        ).await;
                    },
                    "back" => state = SetupState::Main,
                    "done" => state = SetupState::Done,
                    _ => {}
                }
            },
            None => break,
        }
    }
    Ok(())
}

enum SetupState {
    Main,
    Status,
    Logs,
    Security,
    Community,
    Advanced,
    Done,
}

pub fn system_embed(title: &str, desc: &str) -> serenity::CreateEmbed {
    serenity::CreateEmbed::new()
        .title(format!("🛠️ {}", title))
        .description(desc)
        .colour(serenity::Colour::from_rgb(47, 49, 54))
        .footer(serenity::CreateEmbedFooter::new("Head Mod • Unified Control"))
        .timestamp(serenity::Timestamp::now())
}
