-- Add retention policy to guild_configs (default 30 days)
ALTER TABLE guild_configs ADD COLUMN retention_days INTEGER NOT NULL DEFAULT 30;

-- Create table for message archiving (deleted/edited messages)
CREATE TABLE IF NOT EXISTS message_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    content TEXT NOT NULL,
    action_type TEXT NOT NULL, -- 'delete' or 'edit'
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_message_logs_guild_user ON message_logs(guild_id, user_id);
CREATE INDEX IF NOT EXISTS idx_message_logs_created ON message_logs(created_at);
