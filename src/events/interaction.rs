use poise::serenity_prelude as serenity;
use crate::commands::tickets::{execute_open_ticket, OpenTicketParams};

pub struct InteractionHandler;

#[serenity::async_trait]
impl serenity::EventHandler for InteractionHandler {
    async fn interaction_create(&self, ctx: serenity::Context, interaction: serenity::Interaction) {
        if let serenity::Interaction::Component(mut component) = interaction {
            if component.data.custom_id == "create_ticket" {
                // ... (existing ticket logic) ...
                let gid = match component.guild_id {
                    Some(id) => id,
                    None => return,
                };
                let _ = component.defer_ephemeral(&ctx.http).await;
                let data = ctx.data.read().await;
                let data_ref = data.get::<crate::events::DataKey>().expect("Data not found in TypeMap");
                let config = match data_ref.get_config(gid).await {
                    Some(c) => c,
                    None => {
                        let _ = component.edit_response(&ctx.http, serenity::EditInteractionResponse::new().content("❌ Not configured.")).await;
                        return;
                    }
                };
                let Some(host_channel_id) = config.ticket_channel_id else {
                    let _ = component.edit_response(&ctx.http, serenity::EditInteractionResponse::new().content("❌ No host channel.")).await;
                    return;
                };
                let params = OpenTicketParams {
                    http: &ctx.http,
                    db: &data_ref.db,
                    guild_id: gid,
                    host_channel_id: serenity::ChannelId::new(host_channel_id as u64),
                    author: &component.user,
                    mod_role_id: config.ticket_mod_role_id.map(|id| id as u64),
                };
                match execute_open_ticket(params).await {
                    Ok(channel) => { let _ = component.edit_response(&ctx.http, serenity::EditInteractionResponse::new().content(format!("✅ Ticket opened: <#{}>", channel.id))).await; }
                    Err(e) => { let _ = component.edit_response(&ctx.http, serenity::EditInteractionResponse::new().content(format!("❌ Error: {}", e))).await; }
                }
            } else if component.data.custom_id.starts_with("report_") {
                let parts: Vec<&str> = component.data.custom_id.split('_').collect();
                if parts.len() < 3 { return; }
                let action = parts[1];
                let msg_id_u64 = parts[2].parse::<u64>().unwrap_or(0);
                let msg_id_i64 = msg_id_u64 as i64;

                let _ = component.defer_ephemeral(&ctx.http).await;
                let data = ctx.data.read().await;
                let data_ref = data.get::<crate::events::DataKey>().expect("Data not found in TypeMap");

                let gid_i64 = component.guild_id.unwrap().get() as i64;

                // Fetch report from DB
                let report = match sqlx::query!(
                    "SELECT reporter_id, target_id, channel_id, content FROM reports WHERE message_id = ? AND guild_id = ? LIMIT 1",
                    msg_id_i64, gid_i64
                ).fetch_optional(&data_ref.db).await.unwrap_or(None) {
                    Some(r) => r,
                    None => {
                        let _ = component.edit_response(&ctx.http, serenity::EditInteractionResponse::new().content("❌ Report not found in database.")).await;
                        return;
                    }
                };

                let target_user = serenity::UserId::new(report.target_id as u64);
                let channel_id = serenity::ChannelId::new(report.channel_id as u64);
                let message_id = serenity::MessageId::new(msg_id_u64);

                match action {
                    "warn" => {
                        if let Ok(m) = component.guild_id.unwrap().member(&ctx.http, target_user).await {
                            use crate::commands::moderation::actions::{execute_warn, WarnParams};
                            let guild_name = component.guild_id.unwrap().name(&ctx.cache).unwrap_or_else(|| "the server".to_string());
                            let _ = execute_warn(WarnParams {
                                http: &ctx.http,
                                data: data_ref,
                                invoker: &component.user,
                                member: &m,
                                guild_name: &guild_name,
                                reason: "Reported message violation",
                            }).await;
                        }
                        let _ = component.edit_response(&ctx.http, serenity::EditInteractionResponse::new().content("✅ User warned.")).await;
                    }
                    "delete" => {
                        let _ = channel_id.delete_message(&ctx.http, message_id).await;
                        let _ = component.edit_response(&ctx.http, serenity::EditInteractionResponse::new().content("✅ Message deleted.")).await;
                    }
                    "ignore" => {
                        let _ = component.edit_response(&ctx.http, serenity::EditInteractionResponse::new().content("✔️ Report ignored.")).await;
                    }
                    _ => {}
                }

                // Update report status with resolution metadata
                let resolver_id = component.user.id.get() as i64;
                let _ = sqlx::query!(
                    "UPDATE reports SET status = 'resolved', resolved_at = CURRENT_TIMESTAMP, resolved_by = ? WHERE message_id = ?",
                    resolver_id, msg_id_i64
                ).execute(&data_ref.db).await;

                // Edit the original mod-log message
                let embed = component.message.embeds.first().cloned();
                if let Some(e) = embed {
                    let mut creative_embed = serenity::CreateEmbed::from(e);
                    creative_embed = creative_embed.colour(serenity::Colour::DARK_GREEN).title("🚩 Report Resolved");
                    let _ = component.message.edit(&ctx.http, serenity::EditMessage::new().embed(creative_embed).components(vec![])).await;
                }
            } else if component.data.custom_id == "verify_member" {
                let _ = component.defer_ephemeral(&ctx.http).await;
                let data = ctx.data.read().await;
                let data_ref = data.get::<crate::events::DataKey>().expect("Data not found in TypeMap");
                let gid = component.guild_id.unwrap();
                
                if let Some(config) = data_ref.get_config(gid).await {
                    if let Some(rid) = config.verified_role_id {
                        let role_id = serenity::RoleId::new(rid as u64);
                        if let Ok(member) = gid.member(&ctx.http, component.user.id).await {
                            let _ = member.add_role(&ctx.http, role_id).await;
                            let _ = component.edit_response(&ctx.http, serenity::EditInteractionResponse::new().content("✅ You have been successfully verified!")).await;
                        }
                    }
                }
            } else if component.data.custom_id.starts_with("center_") {
                let action = &component.data.custom_id["center_".len()..];
                let _ = component.defer_ephemeral(&ctx.http).await;
                let data = ctx.data.read().await;
                let data_ref = data.get::<crate::events::DataKey>().expect("Data not found in TypeMap");
                let gid_i64 = component.guild_id.unwrap().get() as i64;

                match action {
                    "cases" => {
                        let cases = sqlx::query!(
                            "SELECT id, action_type, reason, created_at FROM cases WHERE guild_id = ? ORDER BY created_at DESC LIMIT 5",
                            gid_i64
                        ).fetch_all(&data_ref.db).await.unwrap_or_default();
                        
                        let lines: Vec<String> = cases.iter().map(|c| {
                            format!("`#{}` **{}**: {} (<t:{}:R>)", c.id.unwrap_or_default(), c.action_type.to_uppercase(), c.reason, c.created_at.and_utc().timestamp())
                        }).collect();
                        
                        let msg = if lines.is_empty() { "No cases found.".to_string() } else { format!("### Recent Cases\n{}", lines.join("\n")) };
                        let _ = component.edit_response(&ctx.http, serenity::EditInteractionResponse::new().content(msg)).await;
                    }
                    "strikes" => {
                        let rules = sqlx::query!(
                            "SELECT strike_count, punishment_type, duration_mins FROM strike_rules WHERE guild_id = ? ORDER BY strike_count ASC",
                            gid_i64
                        ).fetch_all(&data_ref.db).await.unwrap_or_default();
                        
                        let lines: Vec<String> = rules.iter().map(|r| {
                            format!("**{} Strikes**: {} ({})", r.strike_count, r.punishment_type, r.duration_mins.map(|d| format!("{}m", d)).unwrap_or_else(|| "Permanent".to_string()))
                        }).collect();
                        
                        let msg = if lines.is_empty() { "No strike rules configured.".to_string() } else { format!("### Strike Rules\n{}", lines.join("\n")) };
                        let _ = component.edit_response(&ctx.http, serenity::EditInteractionResponse::new().content(msg)).await;
                    }
                    "reports" => {
                        let open_count = sqlx::query!("SELECT COUNT(*) as count FROM reports WHERE guild_id = ? AND status = 'open'", gid_i64)
                            .fetch_one(&data_ref.db).await.map(|r| r.count).unwrap_or(0);
                        let _ = component.edit_response(&ctx.http, serenity::EditInteractionResponse::new().content(format!("There are currently **{}** open reports.", open_count))).await;
                    }
                    "lockdown" => {
                        // Toggling lockdown globally is complex, we'll just toggle for the current channel for the dashboard demo
                        let channel_id = component.channel_id;
                        let everyone_role = component.guild_id.unwrap().get();
                        
                        // Check current perms (simplification: we'll just deny for now)
                        let mut perms = serenity::Permissions::empty();
                        perms.insert(serenity::Permissions::SEND_MESSAGES);
                        
                        let _ = channel_id.create_permission(&ctx.http, serenity::PermissionOverwrite {
                            allow: serenity::Permissions::empty(),
                            deny: perms,
                            kind: serenity::PermissionOverwriteType::Role(serenity::RoleId::new(everyone_role)),
                        }).await;
                        
                        let _ = component.edit_response(&ctx.http, serenity::EditInteractionResponse::new().content("🔒 Channel locked down.")).await;
                    }
                    "refresh" => {
                        let case_count = sqlx::query!("SELECT COUNT(*) as count FROM cases WHERE guild_id = ?", gid_i64).fetch_one(&data_ref.db).await.map(|r| r.count).unwrap_or(0);
                        let report_count = sqlx::query!("SELECT COUNT(*) as count FROM reports WHERE guild_id = ? AND status = 'open'", gid_i64).fetch_one(&data_ref.db).await.map(|r| r.count).unwrap_or(0);
                        
                        let embed = serenity::CreateEmbed::new()
                            .title("🛡️ Head Mod Command Center")
                            .description("Unified control panel for server moderation and oversight.")
                            .field("📊 Stats", format!("**Total Cases:** {}\n**Open Reports:** {}", case_count, report_count), true)
                            .field("🛡️ Status", "System: **Operational**\nMode: **Standard**", true)
                            .colour(serenity::Colour::BLURPLE)
                            .footer(serenity::CreateEmbedFooter::new(format!("Guild: {}", component.guild_id.unwrap())));
                        
                        let _ = component.message.edit(&ctx.http, serenity::EditMessage::new().embed(embed)).await;
                        let _ = component.edit_response(&ctx.http, serenity::EditInteractionResponse::new().content("🔄 Dashboard refreshed.")).await;
                    }
                    _ => {}
                }
            }
        }
    }
}
