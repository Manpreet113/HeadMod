-- Add configuration fields for Phase 7, 8, and 9
ALTER TABLE guild_configs ADD COLUMN toxicity_threshold INTEGER DEFAULT 0;
ALTER TABLE guild_configs ADD COLUMN evidence_channel_id INTEGER;
ALTER TABLE guild_configs ADD COLUMN verification_channel_id INTEGER;
ALTER TABLE guild_configs ADD COLUMN verified_role_id INTEGER;
