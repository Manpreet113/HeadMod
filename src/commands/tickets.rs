use poise::serenity_prelude as serenity;
use crate::{Context, Error};

/// Ticket management commands.
#[poise::command(
    slash_command, guild_only,
    subcommands("open", "close"),
    description_localized("en-US", "Manage support tickets and private help threads.")
)]
pub async fn ticket(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

fn ticket_embed(title: &str, description: &str, color: serenity::Colour) -> serenity::CreateEmbed {
    serenity::CreateEmbed::new()
        .title(title)
        .description(description)
        .colour(color)
        .timestamp(serenity::Timestamp::now())
}

pub struct OpenTicketParams<'a> {
    pub http: &'a serenity::Http,
    pub db: &'a sqlx::SqlitePool,
    pub guild_id: serenity::GuildId,
    pub host_channel_id: serenity::ChannelId,
    pub author: &'a serenity::User,
    pub mod_role_id: Option<u64>,
}

pub async fn execute_open_ticket(params: OpenTicketParams<'_>) -> Result<serenity::GuildChannel, Error> {
    // 0. Get the next Ticket ID
    let next_id = sqlx::query!(
        "SELECT COALESCE(MAX(id), 0) + 1 as next_id FROM tickets"
    ).fetch_one(params.db).await?.next_id;

    let formatted_id = format!("{:04}", next_id);

    // 1. Create the Private Thread
    let thread_name = format!("ticket-{}", formatted_id);
    
    let builder = serenity::CreateThread::new(thread_name)
        .kind(serenity::ChannelType::PrivateThread)
        .invitable(false);

    let thread = params.host_channel_id.create_thread(params.http, builder).await?;

    // 2. Add the author to the thread
    thread.id.add_thread_member(params.http, params.author.id).await?;

    // 3. Add the moderator role (if configured)
    if let Some(rid) = params.mod_role_id {
        thread.id.send_message(params.http, serenity::CreateMessage::new().content(format!("<@&{}>", rid))).await?;
    }

    // 4. DB entry
    let cid_i64 = thread.id.get() as i64;
    let gid_i64 = params.guild_id.get() as i64;
    let uid_i64 = params.author.id.get() as i64;

    sqlx::query!(
        "INSERT INTO tickets (channel_id, guild_id, user_id, reason) VALUES (?, ?, ?, 'Thread-based ticket')",
        cid_i64, gid_i64, uid_i64
    ).execute(params.db).await?;

    // 5. Welcome Message
    let welcome = serenity::CreateEmbed::new()
        .title(format!("🎫 Ticket #{}", formatted_id))
        .description(format!("Hello <@{}>! Welcome to your private support thread.\n\n**Please explain your problem or request in detail here.** Our staff team has been notified and will assist you shortly.", params.author.id))
        .colour(serenity::Colour::BLURPLE)
        .thumbnail(params.author.avatar_url().unwrap_or_default())
        .footer(serenity::CreateEmbedFooter::new("Type /ticket close to end this session"))
        .timestamp(serenity::Timestamp::now());

    thread.id.send_message(params.http, serenity::CreateMessage::new().embed(welcome)).await?;

    Ok(thread)
}

/// Open a new support ticket.
#[poise::command(slash_command, guild_only, description_localized("en-US", "Open a new private support thread to speak with staff."))]
pub async fn open(
    ctx: Context<'_>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let gid = ctx.guild_id().unwrap();
    let author = ctx.author();

    let config = match ctx.data().get_config(gid).await {
        Some(c) => c,
        None => {
            ctx.send(poise::CreateReply::default().content("❌ This server is not configured for tickets yet. Use `/config ticket setup`.").ephemeral(true)).await?;
            return Ok(());
        }
    };

    let Some(host_channel_id) = config.ticket_channel_id else {
        ctx.send(poise::CreateReply::default().content("❌ Ticket host channel is not set. Use `/config ticket setup`.").ephemeral(true)).await?;
        return Ok(());
    };

    let params = OpenTicketParams {
        http: &ctx.http(),
        db: &ctx.data().db,
        guild_id: gid,
        host_channel_id: serenity::ChannelId::new(host_channel_id as u64),
        author,
        mod_role_id: config.ticket_mod_role_id.map(|id| id as u64),
    };

    match execute_open_ticket(params).await {
        Ok(thread) => {
            let success = ticket_embed("Ticket Created", &format!("Your private thread has been opened: <#{}>", thread.id), serenity::Colour::DARK_GREEN);
            ctx.send(poise::CreateReply::default().embed(success).ephemeral(true)).await?;
        }
        Err(e) => {
            ctx.send(poise::CreateReply::default().content(format!("❌ Failed to create ticket thread: {}", e)).ephemeral(true)).await?;
        }
    }

    Ok(())
}

/// Close and delete the current support ticket thread.
#[poise::command(slash_command, guild_only, description_localized("en-US", "Close and delete the current support thread."))]
pub async fn close(
    ctx: Context<'_>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let cid = ctx.channel_id().get() as i64;

    let ticket = sqlx::query!(
        "SELECT user_id FROM tickets WHERE channel_id = ? AND status = 'open'",
        cid
    ).fetch_optional(&ctx.data().db).await?;

    if ticket.is_none() {
        ctx.say("❌ This channel is not an active ticket.").await?;
        return Ok(());
    }

    sqlx::query!(
        "UPDATE tickets SET status = 'closed' WHERE channel_id = ?",
        cid
    ).execute(&ctx.data().db).await?;

    let embed = ticket_embed("Ticket Closing", "This thread has been marked as closed and will be deleted in 10 seconds.", serenity::Colour::RED);
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    let http = ctx.serenity_context().http.clone();
    let chan_id = ctx.channel_id();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        let _ = chan_id.delete(&http).await;
    });

    Ok(())
}
