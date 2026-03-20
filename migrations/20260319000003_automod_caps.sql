-- Add anti-caps toggle to guild_configs (default disabled)
ALTER TABLE guild_configs ADD COLUMN anti_caps BOOLEAN NOT NULL DEFAULT 0;
