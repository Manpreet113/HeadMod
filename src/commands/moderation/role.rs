use poise::serenity_prelude as serenity;
use crate::{Context, Error};
use chrono::{Utc, Duration};

/// Advanced role and permission management.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MANAGE_ROLES",
    required_bot_permissions   = "MANAGE_ROLES",
    subcommands("add", "remove", "info", "audit"),
    description_localized("en-US", "Manage server roles and audit permissions.")
)]
pub async fn role(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add a role to a member, optionally for a limited time.
#[poise::command(slash_command, guild_only, description_localized("en-US", "Assign a role to a member."))]
pub async fn add(
    ctx: Context<'_>,
    #[description = "Member to give the role to"] member: serenity::Member,
    #[description = "Role to assign"] role: serenity::Role,
    #[description = "Optional duration (e.g. 1h, 7d)"] duration: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;

    // Assignment
    member.add_role(&ctx.http(), role.id).await?;

    let mut response = format!("✅ Added role **{}** to **{}**.", role.name, member.user.name);

    if let Some(dur_str) = duration {
        let (val, unit) = {
            let s = dur_str.trim();
            if s.len() < 2 { return Err("Invalid duration format (e.g. 1h, 7d)".into()); }
            let (v_part, u_part) = s.split_at(s.len() - 1);
            let v = v_part.parse::<i64>().map_err(|_| "Invalid duration number")?;
            (v, u_part.to_lowercase())
        };

        let delta = match unit.as_str() {
            "m" => Duration::minutes(val),
            "h" => Duration::hours(val),
            "d" => Duration::days(val),
            _ => return Err("Invalid duration unit (use m, h, d)".into()),
        };

        let expires_at = Utc::now() + delta;

        // Save to DB
        let gid = ctx.guild_id().unwrap().get() as i64;
        let tid = member.user.id.get() as i64;
        let rid = role.id.get() as i64;

        sqlx::query!(
            "INSERT INTO temporary_roles (guild_id, user_id, role_id, expires_at) VALUES (?, ?, ?, ?)",
            gid, tid, rid, expires_at
        ).execute(&ctx.data().db).await?;

        response.push_str(&format!(" This role will expire <t:{}:R>.", expires_at.timestamp()));
    }

    ctx.say(response).await?;
    Ok(())
}

/// Remove a role from a member.
#[poise::command(slash_command, guild_only, description_localized("en-US", "Remove a role from a member."))]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Member to remove the role from"] member: serenity::Member,
    #[description = "Role to remove"] role: serenity::Role,
) -> Result<(), Error> {
    ctx.defer().await?;
    member.remove_role(&ctx.http(), role.id).await?;

    // Also remove any pending temp roles for this combination
    let gid = ctx.guild_id().unwrap().get() as i64;
    let tid = member.user.id.get() as i64;
    let rid = role.id.get() as i64;
    sqlx::query!(
        "DELETE FROM temporary_roles WHERE guild_id = ? AND user_id = ? AND role_id = ?",
        gid, tid, rid
    ).execute(&ctx.data().db).await?;

    ctx.say(format!("✅ Removed role **{}** from **{}**.", role.name, member.user.name)).await?;
    Ok(())
}

/// Show detailed information about a role.
#[poise::command(slash_command, guild_only, description_localized("en-US", "View detailed role metadata and permissions."))]
pub async fn info(
    ctx: Context<'_>,
    #[description = "Role to inspect"] role: serenity::Role,
) -> Result<(), Error> {
    let perms: Vec<String> = role.permissions.get_permission_names().iter().map(|&s| s.to_owned()).collect();

    let embed = serenity::CreateEmbed::new()
        .title(format!("Role Info: {}", role.name))
        .colour(role.colour)
        .field("ID", format!("`{}`", role.id), true)
        .field("Position", format!("{}", role.position), true)
        .field("Mentionable", format!("{}", role.mentionable), true)
        .field("Permissions", if perms.is_empty() { "None".to_string() } else { perms.join(", ") }, false)
        .timestamp(role.id.created_at());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Audit the server for dangerous role permissions.
#[poise::command(slash_command, guild_only, description_localized("en-US", "Scan for high-risk roles and permissions."))]
pub async fn audit(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let guild = ctx.guild_id().unwrap().to_partial_guild(&ctx.http()).await?;
    
    let mut dangerous_roles = Vec::new();
    for role in guild.roles.values() {
        if role.permissions.contains(serenity::Permissions::ADMINISTRATOR) {
            dangerous_roles.push(format!("🔴 **{}** (Administrator)", role.name));
        } else if role.permissions.intersects(serenity::Permissions::MANAGE_GUILD | serenity::Permissions::MANAGE_CHANNELS | serenity::Permissions::MANAGE_ROLES) {
            dangerous_roles.push(format!("🟡 **{}** (High-Level Management)", role.name));
        }
    }

    let description = if dangerous_roles.is_empty() {
        "No high-risk roles detected. Your permission hierarchy looks solid!".to_string()
    } else {
        format!("The following roles have elevated permissions:\n\n{}", dangerous_roles.join("\n"))
    };

    let embed = serenity::CreateEmbed::new()
        .title("🛡️ Permission Audit Results")
        .description(description)
        .colour(if dangerous_roles.is_empty() { serenity::Colour::DARK_GREEN } else { serenity::Colour::ORANGE });

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}
