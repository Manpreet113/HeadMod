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
use crate::types::{Data, Error, WarnEntry};

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
    Ok { message: String, auto_timeout: bool },
    DiscordError(serenity::Error),
    InvalidInput(String),
}

// ── DM helper ─────────────────────────────────────────────────────────────────

async fn dm(http: &serenity::Http, user: &serenity::User, content: String) {
    let _ = user.dm(http, serenity::CreateMessage::new().content(content)).await;
}

// ── execute_kick ──────────────────────────────────────────────────────────────

pub struct KickParams<'a> {
    pub http:       &'a serenity::Http,
    pub data:       &'a Data,
    pub invoker:    &'a serenity::User,
    pub member:     &'a serenity::Member,
    pub guild_name: &'a str,          // extracted from CacheRef by the command
    pub reason:     &'a str,
}

pub async fn execute_kick(p: KickParams<'_>) -> ActionResult {
    dm(p.http, &p.member.user, format!(
        "You have been kicked from **{}**.\nReason: {}", p.guild_name, p.reason,
    )).await;

    match p.member.kick_with_reason(p.http, p.reason).await {
        Ok(()) => {
            log_mod_action(p.http, p.data, p.invoker, &p.member.user,
                ModAction::Kick { reason: p.reason }).await;
            ActionResult::Ok {
                message:      format!("👢 **{}** has been kicked.\n**Reason:** {}", p.member.user.name, p.reason),
                auto_timeout: false,
            }
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
    dm(p.http, &p.member.user, format!(
        "You have been banned from **{}**.\nReason: {}", p.guild_name, p.reason,
    )).await;

    match p.member.ban_with_reason(p.http, p.delete_days, p.reason).await {
        Ok(()) => {
            log_mod_action(p.http, p.data, p.invoker, &p.member.user,
                ModAction::Ban { reason: p.reason, delete_days: p.delete_days }).await;
            ActionResult::Ok {
                message:      format!("🔨 **{}** has been banned.\n**Reason:** {}", p.member.user.name, p.reason),
                auto_timeout: false,
            }
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
            log_mod_action(p.http, p.data, p.invoker, &p.member.user,
                ModAction::Timeout { reason: p.reason, duration: p.duration }).await;
            Ok(ActionResult::Ok {
                message:      format!("⏱️ **{}** timed out for **{}**.\n**Reason:** {}", p.member.user.name, p.duration, p.reason),
                auto_timeout: false,
            })
        }
        Err(e) => Ok(ActionResult::DiscordError(e)),
    }
}

// ── execute_warn ──────────────────────────────────────────────────────────────

pub struct WarnParams<'a> {
    pub http:    &'a serenity::Http,
    pub data:    &'a Data,
    pub invoker: &'a serenity::User,
    pub member:  &'a serenity::Member,
    pub reason:  &'a str,
}

pub async fn execute_warn(p: WarnParams<'_>) -> Result<ActionResult, Error> {
    let warn_count = {
        let mut warns = p.data.warns.entry(p.member.user.id).or_default();
        warns.push(WarnEntry {
            reason:    p.reason.to_owned(),
            moderator: p.invoker.id,
            timestamp: Utc::now(),
        });
        warns.len()
    };

    let mut auto_timeout = false;

    if warn_count >= p.data.warn_threshold {
        let until = Utc::now()
            .checked_add_signed(chrono::Duration::seconds(p.data.warn_timeout_secs))
            .ok_or("Duration overflow")?;
        let ts   = serenity::Timestamp::from_unix_timestamp(until.timestamp())?;
        let edit = serenity::EditMember::new().disable_communication_until(ts.to_string());

        match p.member.guild_id.edit_member(p.http, p.member.user.id, edit).await {
            Ok(_)  => {
                auto_timeout = true;
                tracing::info!(user = %p.member.user.id, warns = warn_count,
                    "Auto-timeout applied after warn threshold");
            }
            Err(e) => tracing::error!("Auto-timeout failed for {}: {}", p.member.user.id, e),
        }
    }

    dm(p.http, &p.member.user, if auto_timeout {
        format!("You have been warned.\n**Reason:** {}\n**Total:** {}\nAuto-timeout applied.", p.reason, warn_count)
    } else {
        format!("You have been warned.\n**Reason:** {}\n**Total:** {}", p.reason, warn_count)
    }).await;

    log_mod_action(p.http, p.data, p.invoker, &p.member.user,
        ModAction::Warn { reason: p.reason, warn_count, auto_timeout }).await;

    Ok(ActionResult::Ok {
        message: format!(
            "⚠️ **{}** warned. Total: **{}**{}",
            p.member.user.name, warn_count,
            if auto_timeout { "\n⏱️ Auto-timeout applied." } else { "" },
        ),
        auto_timeout,
    })
}