-- Community Engagement & Alerts
ALTER TABLE guild_configs ADD COLUMN join_log_channel_id INTEGER;
ALTER TABLE guild_configs ADD COLUMN leave_log_channel_id INTEGER;
ALTER TABLE guild_configs ADD COLUMN suspicious_log_channel_id INTEGER;
ALTER TABLE guild_configs ADD COLUMN global_intel_enabled BOOLEAN NOT NULL DEFAULT 1;
