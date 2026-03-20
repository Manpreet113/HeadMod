# 🛡️ Head Mod

**Head Mod** is a premium, unified security and moderation powerhouse for Discord servers. Designed with a focus on professional server management and a streamlined user experience, it replaces fragmented configuration commands with a sophisticated, interactive "God-Mode" setup.

![Head Mod Logo](temp.png)

## 🚀 Key Features

### 🛠️ Unified `/setup` Wizard
The crown jewel of Head Mod. A single interactive command hub that allows administrators to configure:
- **Logging**: Message logs, join/leave events, and audit archives.
- **Security**: Global intelligence toggles, anti-invite, anti-spam, and toxicity thresholds.
- **Community**: Automated verification systems and ticket support.
- **Advanced**: System maintenance and database cleanup.

### 🎨 Professional Embed Builder
Create stunning, high-end announcements with the `/embed` command. Features include:
- Real-time interactive previews.
- Complete customization of titles, descriptions, colors, fields, and footers.
- Integration of custom branding assets.

### 🧠 Advanced Protection
- **Global Intelligence**: Cross-server security checks identify known bad actors before they cause trouble.
- **Neural Toxicity Filter**: Fine-grained sentiment analysis with per-channel overrides and "Relaxed Mode" for community zones.
- **Humorous Alerts**: Security notifications that are as witty as they are informative.

### 📊 Automated Insights
- **Weekly Health Summaries**: Every Sunday, Head Mod delivers a comprehensive report on moderation activity and server security health.

## 🛠️ Getting Started

### Prerequisites
- [Rust](https://www.rust-lang.org/) (Latest stable)
- [SQLX CLI](https://github.com/launchbadge/sqlx) (Optional, for database migrations)
- SQLite

### Environment Variables
Create a `.env` file in the root directory:
```env
DISCORD_TOKEN=your_token_here
DATABASE_URL=sqlite://data.db
GUILD_ID=your_development_guild_id (Optional)
```

### Installation
1. Clone the repository.
2. Initialize the database:
   ```bash
   sqlx db create
   sqlx migrate run
   ```
3. Run the bot:
   ```bash
   cargo run --release
   ```

## 📂 Project Structure
- `src/main.rs`: Application entry point and framework setup.
- `src/commands/`: Command implementations (Configuration, Moderation, Utility).
- `src/events/`: Event handlers (AutoMod, Member tracking, Interactions).
- `src/types.rs`: Core data structures and database models.
- `src/logging/`: Centralized logging and case management.

## 🍷 Branding
Head Mod is built for consistency. All professional assets (`temp.png` and `temp2.png`) are integrated into the core experience to provide a premium look and feel.

---
*Built with ❤️ using [Poise](https://github.com/serenity-rs/poise) and [Serenity](https://github.com/serenity-rs/serenity).*
