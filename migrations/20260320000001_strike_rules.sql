-- Table to store custom strike escalation rules
CREATE TABLE IF NOT EXISTS strike_rules (
    guild_id INTEGER NOT NULL,
    strike_count INTEGER NOT NULL, -- Number of warns required
    punishment_type TEXT NOT NULL, -- 'Timeout', 'Kick', 'Ban'
    duration_mins INTEGER, -- Duration for timeouts (optional for Kick/Ban)
    PRIMARY KEY (guild_id, strike_count)
);

-- Insert some sensible defaults for new guilds (can be overridden)
-- Note: Logic in code should handle the absence of rules too.
