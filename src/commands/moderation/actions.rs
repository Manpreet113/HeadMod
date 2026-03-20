//! Shared moderation logic.
//!
//! Commands are responsible for:
//!   1. Input validation
//!   2. Extracting plain data from CacheRef (hierarchy check, guild name)
//!   3. Calling an execute_* function here
//!   4. Replying based on the returned ActionResult
//!
//! execute_* functions receive only owned/plain values — no CacheRef ever
//! crosses an .await point inside this module.

use chrono::Utc;
use poise::serenity_prelude as serenity;
use crate::logging::{log_mod_action, ModAction};
use crate::types::{Data, Error};

// ── Hierarchy check ───────────────────────────────────────────────────────────

/// Returns `(invoker_can_act, bot_can_act)`.
/// Call this inside a plain sync block and drop the CacheRef before any await.
pub fn hierarchy_check(
    guild:      &serenity::Guild,
    invoker_id: serenity::UserId,
    bot_id:     serenity::UserId,
    target_id:  serenity::UserId,
) -> (bool, bool) {
    if target_id == guild.owner_id { return (false, false); }

    let top = |uid: serenity::UserId| -> i64 {
        guild.members.get(&uid)
            .and_then(|m| guild.member_highest_role(m))
            .map(|r| r.position.into())
            .unwrap_or(0)
    };

    let target_pos  = top(target_id);
    let invoker_can = invoker_id == guild.owner_id || top(invoker_id) > target_pos;
    let bot_can     = top(bot_id) > target_pos;
    (invoker_can, bot_can)
}

// ── Duration parsing ──────────────────────────────────────────────────────────

/// Parse `10m`, `2h`, `7d` into seconds. Returns `None` on bad input.
pub fn parse_duration(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() { return None; }
    let (num, unit) = s.split_at(s.len().checked_sub(1)?);
    let n: i64 = num.parse().ok()?;
    if n <= 0 { return None; }
    match unit {
        "s" => Some(n),
        "m" => Some(n * 60),
        "h" => Some(n * 3_600),
        "d" => Some(n * 86_400),
        _   => None,
    }
}

// ── Action result ─────────────────────────────────────────────────────────────

/// Outcome of an execute_* call. The command turns this into a Discord reply.
pub enum ActionResult {
    Ok(serenity::CreateEmbed),
    DiscordError(serenity::Error),
    InvalidInput(String),
}

fn mod_embed(title: &str, color: serenity::Colour) -> serenity::CreateEmbed {
    serenity::CreateEmbed::new()
        .title(title)
        .colour(color)
        .timestamp(serenity::Timestamp::now())
}

// ── DM helper ─────────────────────────────────────────────────────────────────

async fn dm(http: &serenity::Http, user: &serenity::User, content: String) {
    let _ = user.dm(http, serenity::CreateMessage::new().content(content)).await;
}

// ── execute_kick ──────────────────────────────────────────────────────────────

pub async fn check_anti_nuke(data: &Data, guild_id: serenity::GuildId, moderator_id: serenity::UserId) -> Result<bool, Error> {
    let gid = guild_id.get() as i64;
    let mid = moderator_id.get() as i64;
    let five_mins_ago = chrono::Utc::now() - chrono::Duration::minutes(5);
    
    let row = sqlx::query!(
        "SELECT COUNT(*) as count FROM cases WHERE guild_id = ? AND moderator_id = ? AND action_type IN ('ban', 'kick', 'timeout') AND created_at >= ?",
        gid, mid, five_mins_ago
    ).fetch_one(&data.db).await?;
    
    // Threshold is 5 actions per 5 minutes.
    Ok(row.count >= 5)
}

pub struct KickParams<'a> {
    pub http:       &'a serenity::Http,
    pub data:       &'a Data,
    pub invoker:    &'a serenity::User,
    pub member:     &'a serenity::Member,
    pub guild_name: &'a str,
    pub reason:     &'a str,
}

pub async fn execute_kick(p: KickParams<'_>) -> ActionResult {
    if check_anti_nuke(p.data, p.member.guild_id, p.invoker.id).await.unwrap_or(false) {
        return ActionResult::InvalidInput("Anti-nuke: You are issuing too many actions too quickly.".into());
    }

    dm(p.http, &p.member.user, format!(
        "You have been kicked from **{}**.\nReason: {}", p.guild_name, p.reason,
    )).await;

    match p.member.kick_with_reason(p.http, p.reason).await {
        Ok(()) => {
            let case_id = log_mod_action(p.http, p.data, p.member.guild_id, p.invoker, &p.member.user,
                ModAction::Kick { reason: p.reason }).await;
            ActionResult::Ok(mod_embed("👢 User Kicked", serenity::Colour::RED)
                .field("User", format!("{} (`{}`)", p.member.user.name, p.member.user.id), false)
                .field("Moderator", format!("{} (`{}`)", p.invoker.name, p.invoker.id), false)
                .field("Reason", p.reason, false)
                .footer(serenity::CreateEmbedFooter::new(format!("Case #{}", case_id))))
        }
        Err(e) => ActionResult::DiscordError(e),
    }
}

// ── execute_ban ───────────────────────────────────────────────────────────────

pub struct BanParams<'a> {
    pub http:        &'a serenity::Http,
    pub data:        &'a Data,
    pub invoker:     &'a serenity::User,
    pub member:      &'a serenity::Member,
    pub guild_name:  &'a str,
    pub reason:      &'a str,
    pub delete_days: u8,
}

pub async fn execute_ban(p: BanParams<'_>) -> ActionResult {
    if check_anti_nuke(p.data, p.member.guild_id, p.invoker.id).await.unwrap_or(false) {
        return ActionResult::InvalidInput("Anti-nuke: You are issuing too many actions too quickly.".into());
    }

    dm(p.http, &p.member.user, format!(
        "You have been banned from **{}**.\nReason: {}", p.guild_name, p.reason,
    )).await;

    match p.member.ban_with_reason(p.http, p.delete_days, p.reason).await {
        Ok(()) => {
            let case_id = log_mod_action(p.http, p.data, p.member.guild_id, p.invoker, &p.member.user,
                ModAction::Ban { reason: p.reason, delete_days: p.delete_days }).await;
            let embed = serenity::CreateEmbed::new()
                .title("🔨 Member Banned")
                .colour(serenity::Colour::DARK_RED)
                .thumbnail(p.member.user.face())
                .field("User", format!("<@{}>", p.member.user.id), true)
                .field("Moderator", format!("<@{}>", p.invoker.id), true)
                .field("Reason", p.reason, false)
                .field("Messages Deleted", format!("{} days", p.delete_days), true)
                .footer(serenity::CreateEmbedFooter::new(format!("Case #{}", case_id)))
                .timestamp(serenity::Timestamp::now());
            ActionResult::Ok(embed)
        }
        Err(e) => ActionResult::DiscordError(e),
    }
}

// ── execute_timeout ───────────────────────────────────────────────────────────

pub struct TimeoutParams<'a> {
    pub http:     &'a serenity::Http,
    pub data:     &'a Data,
    pub invoker:  &'a serenity::User,
    pub member:   &'a serenity::Member,
    pub reason:   &'a str,
    pub duration: &'a str,
}

pub async fn execute_timeout(p: TimeoutParams<'_>) -> Result<ActionResult, Error> {
    if check_anti_nuke(p.data, p.member.guild_id, p.invoker.id).await.unwrap_or(false) {
        return Ok(ActionResult::InvalidInput("Anti-nuke: You are issuing too many actions too quickly.".into()));
    }

    let secs = match parse_duration(p.duration) {
        Some(s) => s,
        None    => return Ok(ActionResult::InvalidInput(
            "Invalid duration. Use a number followed by `s`, `m`, `h`, or `d` (e.g. `10m`).".into()
        )),
    };

    const MAX_SECS: i64 = 28 * 24 * 3_600;
    if secs > MAX_SECS {
        return Ok(ActionResult::InvalidInput("Maximum timeout duration is 28 days.".into()));
    }

    let until = Utc::now()
        .checked_add_signed(chrono::Duration::seconds(secs))
        .ok_or("Duration overflow")?;
    let ts = serenity::Timestamp::from_unix_timestamp(until.timestamp())?;
    let edit = serenity::EditMember::new().disable_communication_until(ts.to_string());

    match p.member.guild_id.edit_member(p.http, p.member.user.id, edit).await {
        Ok(_) => {
            let case_id = log_mod_action(p.http, p.data, p.member.guild_id, p.invoker, &p.member.user,
                ModAction::Timeout { reason: p.reason, duration: p.duration }).await;
            let embed = serenity::CreateEmbed::new()
                .title("⏳ Member Timed Out")
                .colour(serenity::Colour::GOLD)
                .thumbnail(p.member.user.face())
                .field("User", format!("<@{}>", p.member.user.id), true)
                .field("Moderator", format!("<@{}>", p.invoker.id), true)
                .field("Duration", p.duration, true)
                .field("Reason", p.reason, false)
                .footer(serenity::CreateEmbedFooter::new(format!("Case #{}", case_id)))
                .timestamp(serenity::Timestamp::now());
            Ok(ActionResult::Ok(embed))
        }
        Err(e) => Ok(ActionResult::DiscordError(e)),
    }
}

// ── execute_warn ──────────────────────────────────────────────────────────────

pub struct WarnParams<'a> {
    pub http:       &'a serenity::Http,
    pub data:       &'a Data,
    pub invoker:    &'a serenity::User,
    pub member:     &'a serenity::Member,
    pub guild_name: &'a str,
    pub reason:     &'a str,
}

pub async fn execute_warn(p: WarnParams<'_>) -> Result<ActionResult, Error> {
    let gid = p.member.guild_id.get() as i64;
    let tid = p.member.user.id.get() as i64;

    let row = sqlx::query!("SELECT COUNT(*) as count FROM cases WHERE guild_id = ? AND target_id = ? AND action_type = 'warn'", gid, tid)
        .fetch_one(&p.data.db).await?;
    let warn_count = (row.count + 1) as usize;

    let warn_count_i64 = warn_count as i64;
    let rules = sqlx::query!(
        "SELECT punishment_type, duration_mins FROM strike_rules WHERE guild_id = ? AND strike_count = ?",
        gid, warn_count_i64
    ).fetch_optional(&p.data.db).await?;

    let mut auto_action = None;
    let mut auto_action_desc = String::new();

    if let Some(rule) = rules {
        match rule.punishment_type.to_lowercase().as_str() {
            "timeout" => {
                let mins = rule.duration_mins.unwrap_or(60);
                let until = Utc::now() + chrono::Duration::minutes(mins);
                if let Ok(ts) = serenity::Timestamp::from_unix_timestamp(until.timestamp()) {
                    let edit = serenity::EditMember::new().disable_communication_until(ts.to_string());
                    if let Ok(_) = p.member.guild_id.edit_member(p.http, p.member.user.id, edit).await {
                        auto_action = Some("Timeout");
                        auto_action_desc = format!("⏱️ Applied a **{} minute** timeout.", mins);
                    }
                }
            }
            "kick" => {
                if let Ok(_) = p.member.kick_with_reason(p.http, "Exceeded warning strike threshold").await {
                    auto_action = Some("Kick");
                    auto_action_desc = "👞 Kicked from the server.".to_string();
                }
            }
            "ban" => {
                if let Ok(_) = p.member.ban_with_reason(p.http, 0, "Exceeded warning strike threshold").await {
                    auto_action = Some("Ban");
                    auto_action_desc = "🔨 Permanently banned.".to_string();
                }
            }
            _ => {}
        }
    } else {
        // Fallback to legacy guild_configs threshold if no specific strike rule exists
        let (warn_threshold, warn_timeout_secs) = match sqlx::query!("SELECT warn_threshold, warn_timeout_secs FROM guild_configs WHERE guild_id = ?", gid).fetch_optional(&p.data.db).await? {
            Some(r) => (r.warn_threshold as usize, r.warn_timeout_secs as i64),
            None => (3, 3600),
        };

        if warn_count >= warn_threshold {
            let until = Utc::now() + chrono::Duration::seconds(warn_timeout_secs);
            if let Ok(ts) = serenity::Timestamp::from_unix_timestamp(until.timestamp()) {
                let edit = serenity::EditMember::new().disable_communication_until(ts.to_string());
                if let Ok(_) = p.member.guild_id.edit_member(p.http, p.member.user.id, edit).await {
                    auto_action = Some("Timeout");
                    auto_action_desc = format!("⏱️ Applied a **{} minute** timeout (default threshold).", warn_timeout_secs / 60);
                }
            }
        }
    }

    dm(p.http, &p.member.user, format!(
        "You have been warned in **{}**.\n**Reason:** {}\n**Total Warnings:** {}\n{}",
        p.guild_name, p.reason, warn_count, auto_action_desc
    )).await;

    let case_id = log_mod_action(p.http, p.data, p.member.guild_id, p.invoker, &p.member.user,
        ModAction::Warn { reason: p.reason, warn_count, auto_timeout: auto_action.is_some() }).await;

    let mut embed = serenity::CreateEmbed::new()
        .title("⚠️ Member Warned")
        .colour(serenity::Colour::ORANGE)
        .thumbnail(p.member.user.face())
        .field("User", format!("<@{}>", p.member.user.id), true)
        .field("Moderator", format!("<@{}>", p.invoker.id), true)
        .field("Reason", p.reason, false)
        .footer(serenity::CreateEmbedFooter::new(format!("Case #{}", case_id)))
        .timestamp(serenity::Timestamp::now())
        .field("Total Warnings", warn_count.to_string(), true);
    
    if let Some(action) = auto_action {
        embed = embed.field("Auto-Action", format!("{} threshold met", action), false);
        if !auto_action_desc.is_empty() {
            embed = embed.description(&auto_action_desc);
        }
    }

    Ok(ActionResult::Ok(embed))
}

// ── execute_unban ─────────────────────────────────────────────────────────────

pub struct UnbanParams<'a> {
    pub http:     &'a serenity::Http,
    pub data:     &'a Data,
    pub invoker:  &'a serenity::User,
    pub guild_id: serenity::GuildId,
    pub user_id:  serenity::UserId,
    pub reason:   &'a str,
}

pub async fn execute_unban(p: UnbanParams<'_>) -> ActionResult {
    match p.guild_id.unban(p.http, p.user_id).await {
        Ok(_)  => {
            let uid_i64 = p.user_id.get() as i64;
            let mid_i64 = p.invoker.id.get() as i64;
            let gid_i64 = p.guild_id.get() as i64;

            let case_id = sqlx::query!(
                "INSERT INTO cases (guild_id, moderator_id, target_id, action_type, reason) VALUES (?, ?, ?, 'unban', ?)",
                gid_i64, mid_i64, uid_i64, p.reason
            ).execute(&p.data.db).await.map(|r| r.last_insert_rowid() as u64).unwrap_or(0);

            ActionResult::Ok(mod_embed("🔓 User Unbanned", serenity::Colour::DARK_GREEN)
                .field("User ID", p.user_id.get().to_string(), false)
                .field("Moderator", format!("{} (`{}`)", p.invoker.name, p.invoker.id), false)
                .field("Reason", p.reason, false)
                .footer(serenity::CreateEmbedFooter::new(format!("Case #{}", case_id))))
        }
        Err(e) => ActionResult::DiscordError(e),
    }
}