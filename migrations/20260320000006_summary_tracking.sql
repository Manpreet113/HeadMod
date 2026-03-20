-- Track when the last weekly summary was sent to avoid duplicate posts
ALTER TABLE guild_configs ADD COLUMN last_summary_at DATETIME;
