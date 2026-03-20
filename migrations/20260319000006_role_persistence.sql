-- Table to store roles that should be re-applied when a user rejoins
CREATE TABLE IF NOT EXISTS persistent_roles (
    guild_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    role_id INTEGER NOT NULL,
    PRIMARY KEY (guild_id, user_id, role_id)
);

-- Table to store active timeouts that should be re-applied when a user rejoins
CREATE TABLE IF NOT EXISTS persistent_timeouts (
    guild_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    timeout_until TEXT NOT NULL, -- ISO8601 string
    PRIMARY KEY (guild_id, user_id)
);

-- Add anti_evasion toggle to guild_configs
ALTER TABLE guild_configs ADD COLUMN anti_evasion BOOLEAN NOT NULL DEFAULT 0;
