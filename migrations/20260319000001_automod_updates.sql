-- Add automod toggles to guild_configs
ALTER TABLE guild_configs ADD COLUMN anti_invite BOOLEAN NOT NULL DEFAULT 1;
ALTER TABLE guild_configs ADD COLUMN anti_spam BOOLEAN NOT NULL DEFAULT 1;

-- Create table for blacklisted words
CREATE TABLE IF NOT EXISTS blacklisted_words (
    guild_id BIGINT NOT NULL,
    word TEXT NOT NULL,
    PRIMARY KEY (guild_id, word)
);

CREATE INDEX IF NOT EXISTS idx_blacklisted_words_guild ON blacklisted_words(guild_id);
