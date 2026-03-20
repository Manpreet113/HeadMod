-- Neural Customization: Per-Channel AI Overrides
CREATE TABLE IF NOT EXISTS channel_configs (
    channel_id INTEGER PRIMARY KEY,
    guild_id INTEGER NOT NULL,
    toxicity_threshold INTEGER NOT NULL DEFAULT 0, -- 0 = use global, >0 = override
    is_relaxed BOOLEAN NOT NULL DEFAULT 0,
    FOREIGN KEY(guild_id) REFERENCES guild_configs(guild_id)
);
