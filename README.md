# 🛡️ Head Mod

> *A Discord moderation bot built with Rust, Poise, and an increasingly concerning amount of ambition.*

---

> [!CAUTION]
> **Proceed with appropriate levels of skepticism.**
>
> A significant portion of this bot is still experimental. Features may work perfectly, partially, or in ways that were not originally intended but somehow ended up in production anyway. Several subsystems — particularly parts of the background job scheduler — were implemented with AI assistance for the bits that were genuinely beyond the current author's (my) grasp. They work. Mostly. I think. Please don't bet your server on it just yet.
>
> Additionally: a web dashboard has been on the wishlist since approximately day three of this project. The vision is clear. The integration path is not. Until I figure out how to cleanly bridge a Rust Discord bot backend with a web frontend without losing my mind, the dashboard remains a dream deferred — an unchecked box that exists both in the README and in the codebase.

---

## What Is This?

Head Mod is a full-stack Discord moderation bot written in Rust using [Poise](https://github.com/serenity-rs/poise) and [Serenity](https://github.com/serenity-rs/serenity). It started as a simple kick/ban bot and — through a series of increasingly ambitious decisions — evolved into something with a database, a case system, automod, background jobs, a ticket system, a config wizard, and a toxicity filter with a suspiciously confident scoring algorithm.

It is built for servers that want serious moderation tooling without paying for a premium bot subscription, and for developers who want to learn Rust by doing something that will occasionally yell at them with compiler errors about `CacheRef` not being `Send`.

---

## ✨ Features

### 🔨 Core Moderation
Everything you'd expect from a mod bot, done properly:
- `/kick`, `/ban`, `/unban`, `/timeout`, `/tempban` — all with hierarchy checks, DMs to the target, and automatic case logging
- `/warn add` / `/warn list` / `/warn clear` — persistent warnings with configurable auto-escalation per guild
- `/purge` — bulk delete up to 100 messages with the 14-day Discord limit enforced so you don't get a confusing API error

### 📋 Case System
Every moderation action is a case. Every case has a number.
- `/case view 17` — see exactly what happened, when, and who did it
- `/case edit 17` — update the reason after the fact
- `/case delete 17` — remove it from the record (permanently, no undo, you've been warned)
- `/history @user` — full moderation history for any user, most recent first
- `/whois @user` — full user profile with account age, roles, risk level, and recent infractions

### ⚙️ Per-Guild Configuration
Every setting is per-server. Nothing is hardcoded to your `.env` file (well, almost nothing):
- Warn thresholds and escalation policies
- Custom strike rules: set what happens at 3 warns, 5 warns, etc. — timeout, kick, or ban
- Auto-moderation toggles: anti-invite, anti-spam, anti-caps, toxicity filtering
- Alt detection: kick accounts below a minimum age
- Anti-evasion: restore timeouts and roles when a banned user rejoins

### 📊 Logging
Three separate log channels if you want them, one if you don't:
- **Mod log** — every action with a case number and a colour-coded embed
- **Message log** — deleted and edited messages, member joins/leaves, role changes
- **Evidence vault** — attachments from deleted messages, automatically mirrored

### 🤖 Automod
Runs silently in the background judging your members:
- Invite link detection
- Message flood detection (5 messages in 5 seconds → 5 minute timeout)
- Mass mention detection (>5 mentions → immediate deletion)
- Blacklisted word filter (per-guild, stored in the database)
- Caps lock filter (>70% uppercase on messages over 10 characters)
- Toxicity scoring with per-channel threshold overrides and a "relaxed mode" for channels where language norms are looser

### 🎫 Ticket System
Private thread-based support tickets:
- `/ticket open` — creates a private thread, adds the user, pings the mod role
- `/ticket close` — marks closed, waits 10 seconds, deletes the thread
- Report-to-ticket pipeline: right-click any message → Report Message → mods see an embed with Warn / Delete / Ignore buttons

### 📈 Staff Insights
- `/performance` — moderator leaderboard by case count, report resolution stats, average response times
- Weekly automated health summaries posted to your mod log every Sunday at 4 AM UTC

### 🔐 Security
- **Anti-nuke protection** — moderators who issue 5+ actions in 5 minutes get flagged and blocked
- **Raid detection** — alerts fire when more than 10 members join within 60 seconds
- **Global intelligence** — a shared ban network that checks new members against a cross-server list of known bad actors. Useless for a single server
- **Emergency Protocol Alpha** — `/emergency` locks every text channel in the server behind a confirmation button, for when things go very wrong very fast

### 🛠️ Utility
- `/slowmode` — set channel slowmode in seconds
- `/role add/remove/info/audit` — role management with temporary role support and a permission audit scanner
- `/lockdown` / `/unlock` — lock or unlock individual channels or the entire server
- `/announce` — send formatted announcements with optional embed titles and hex colours
- `/center` — an interactive moderation dashboard embed with live stats
- `/setup` — the god-mode config wizard: an interactive button menu for configuring everything

---

## 🚀 Getting Started

### Prerequisites
- [Rust](https://www.rust-lang.org/) (latest stable)
- A PostgreSQL or SQLite database (SQLite by default — no setup required)
- A Discord bot token with the following **Privileged Gateway Intents** enabled in the Developer Portal:
  - Server Members Intent
  - Message Content Intent

### Environment Variables
Create a `.env` file in the root:

```env
DISCORD_TOKEN=your_bot_token_here
DATABASE_URL=sqlite://data.db
GUILD_ID=your_dev_guild_id   # Optional: omit for global command registration
MOD_LOG_CHANNEL_ID=          # Optional: overridden by /setup
MESSAGE_LOG_CHANNEL_ID=      # Optional: overridden by /setup
```

### Installation

```bash
# Clone the repo
git clone https://github.com/manpreet113/head-mod
cd head-mod

# Run migrations (requires sqlx-cli)
cargo install sqlx-cli
sqlx migrate run

# Or use the sync script if sqlx-cli is being difficult
bash sync_db.sh

# Build and run
cargo run --release
```

---

## 📂 Project Structure

```
src/
├── main.rs                      # Entry point, client setup, framework init
├── types.rs                     # Data, GuildConfig, ChannelConfig, shared state
├── logging/
│   └── mod.rs                   # log_mod_action, ModAction, case DB writes
├── events/
│   ├── mod.rs                   # Re-exports all handlers
│   ├── message_log.rs           # Delete/edit logs, join/leave, role change logs
│   ├── automod.rs               # Real-time message scanning
│   ├── interaction.rs           # Button interaction handler
│   ├── member.rs                # Join/leave alerts, suspicious account detection
│   └── persistence.rs           # Anti-evasion: role and timeout restoration
├── commands/
│   ├── mod.rs
│   ├── general.rs               # /ping, /help
│   ├── config.rs                # /setup wizard
│   ├── announcements.rs         # /announce
│   ├── embeds.rs                # /embed builder
│   ├── tickets.rs               # /ticket open, /ticket close
│   └── moderation/
│       ├── mod.rs
│       ├── actions.rs           # Shared logic: hierarchy check, execute_*, ActionResult
│       ├── kick.rs              # /kick
│       ├── ban.rs               # /ban
│       ├── unban.rs             # /unban
│       ├── timeout.rs           # /timeout
│       ├── tempban.rs           # /tempban
│       ├── warn.rs              # /warn add/list/clear
│       ├── purge.rs             # /purge
│       ├── case.rs              # /case view/edit/delete
│       ├── history.rs           # /history
│       ├── whois.rs             # /whois
│       ├── logs.rs              # /logs
│       ├── stats.rs             # /performance
│       ├── mod_center.rs        # /center
│       ├── lockdown.rs          # /lockdown, /unlock
│       ├── emergency.rs         # /emergency
│       ├── role.rs              # /role
│       ├── report.rs            # Right-click report
│       ├── filters.rs           # /channel_filter
│       └── utility.rs           # /slowmode, /purge (with user filter)
├── background_jobs.rs           # Temp role expiry, weekly summary scheduler
migrations/                      # SQL migration files
```

---

## 🗺️ Wishlist / Known Limitations

- **Web Dashboard** — The dream. A proper web interface to manage guild configs, browse cases, review reports, and visualise moderation trends. The backend data is all there in SQLite. The sticking point is bridging a long-running Rust async process with a web server in a way that doesn't feel like duct tape. If you know how to do this cleanly, please open a PR or leave a strongly-worded issue.

- **Warns are now persistent** — stored in the `cases` table. Rejoice.

- **Escalation policies** are stored in `strike_rules` per guild, but the `/setup` wizard doesn't yet expose a UI for editing them. You'll need to insert rows manually or wait for the dashboard that doesn't exist yet.

- **Global intelligence** (`global_bans` table) has no ingestion pipeline. Entries have to be added directly to the database for now. A future `/globalban` admin command is planned.

- **Toxicity scoring** is a heuristic keyword scorer, not a neural network, despite what the README might imply. It will catch obvious cases and miss creative ones. Treat it as a first line of defence, not a final arbiter.

---

## 🤝 Contributing

Issues and PRs are welcome. If you find a bug, please include the tracing output — the bot logs everything with structured fields so the error context should be right there. If you want to add a feature, open an issue first so we can discuss the design before you spend three hours on it.

---

## 📄 License

MIT. Do whatever you want with it. If you use it to moderate a server about Rust programming, that would be poetic and appreciated.

---

*Built with ❤️ (and occasional despair) using [Poise](https://github.com/serenity-rs/poise) and [Serenity](https://github.com/serenity-rs/serenity).*