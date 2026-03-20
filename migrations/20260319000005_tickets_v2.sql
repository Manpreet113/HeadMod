-- Combined Ticket System Migration
ALTER TABLE guild_configs ADD COLUMN ticket_channel_id BIGINT;
ALTER TABLE guild_configs ADD COLUMN ticket_mod_role_id BIGINT;

CREATE TABLE IF NOT EXISTS tickets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    channel_id BIGINT NOT NULL,
    guild_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    reason TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
