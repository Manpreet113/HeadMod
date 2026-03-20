use poise::serenity_prelude as serenity;
use crate::events::message_log::DataKey;

pub struct PersistenceHandler;

#[serenity::async_trait]
impl serenity::EventHandler for PersistenceHandler {
    async fn guild_member_removal(&self, ctx: serenity::Context, guild_id: serenity::GuildId, user: serenity::User, _member_data: Option<serenity::Member>) {
        let data = ctx.data.read().await;
        let Some(bot_data) = data.get::<DataKey>() else { return };

        let config = match bot_data.get_config(guild_id).await {
            Some(c) => c,
            None => return,
        };

        if !config.anti_evasion { return; }

        let member = match guild_id.member(&ctx.http, user.id).await {
            Ok(m) => m,
            Err(_) => return,
        };

        let gid_i64 = guild_id.get() as i64;
        let uid_i64 = user.id.get() as i64;

        // 1. Save Roles
        // We only save roles that the bot can actually re-apply (less than bot's highest role)
        // But for simplicity in this mod bot, we save all non-@everyone roles.
        for role_id in &member.roles {
            let rid_i64 = role_id.get() as i64;
            let _ = sqlx::query!(
                "INSERT INTO persistent_roles (guild_id, user_id, role_id) VALUES (?, ?, ?)
                 ON CONFLICT DO NOTHING",
                gid_i64, uid_i64, rid_i64
            ).execute(&bot_data.db).await;
        }

        // 2. Save Timeout
        if let Some(until) = member.communication_disabled_until {
            let until_str = until.to_rfc3339();
            let _ = sqlx::query!(
                "INSERT INTO persistent_timeouts (guild_id, user_id, timeout_until) VALUES (?, ?, ?)
                 ON CONFLICT(guild_id, user_id) DO UPDATE SET timeout_until = excluded.timeout_until",
                gid_i64, uid_i64, until_str
            ).execute(&bot_data.db).await;
        }
    }

    async fn guild_member_addition(&self, ctx: serenity::Context, new_member: serenity::Member) {
        let data = ctx.data.read().await;
        let Some(bot_data) = data.get::<DataKey>() else { return };

        let guild_id = new_member.guild_id;
        let config = match bot_data.get_config(guild_id).await {
            Some(c) => c,
            None => return,
        };

        if !config.anti_evasion { return; }

        let gid_i64 = guild_id.get() as i64;
        let uid_i64 = new_member.user.id.get() as i64;

        // 1. Restore Roles
        let roles = sqlx::query!(
            "SELECT role_id FROM persistent_roles WHERE guild_id = ? AND user_id = ?",
            gid_i64, uid_i64
        ).fetch_all(&bot_data.db).await.unwrap_or_default();

        for row in roles {
            let role_id = serenity::RoleId::new(row.role_id as u64);
            let _ = new_member.add_role(&ctx.http, role_id).await;
        }
        // Cleanup roles table for this user
        let _ = sqlx::query!("DELETE FROM persistent_roles WHERE guild_id = ? AND user_id = ?", gid_i64, uid_i64).execute(&bot_data.db).await;

        // 2. Restore Timeout
        let timeout = sqlx::query!(
            "SELECT timeout_until FROM persistent_timeouts WHERE guild_id = ? AND user_id = ?",
            gid_i64, uid_i64
        ).fetch_optional(&bot_data.db).await.unwrap_or(None);

        if let Some(row) = timeout {
            if let Ok(until) = serenity::Timestamp::parse(&row.timeout_until) {
                let now = serenity::Timestamp::now();
                if until.unix_timestamp() > now.unix_timestamp() {
                    let edit = serenity::EditMember::new().disable_communication_until(until.to_string());
                    let _ = guild_id.edit_member(&ctx.http, new_member.user.id, edit).await;
                }
            }
            // Cleanup timeout table for this user
            let _ = sqlx::query!("DELETE FROM persistent_timeouts WHERE guild_id = ? AND user_id = ?", gid_i64, uid_i64).execute(&bot_data.db).await;
        }
    }
}
