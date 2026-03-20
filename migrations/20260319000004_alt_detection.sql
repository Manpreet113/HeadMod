-- Add alt account age threshold (default 0 days = disabled)
ALTER TABLE guild_configs ADD COLUMN min_account_age_days INTEGER NOT NULL DEFAULT 0;
