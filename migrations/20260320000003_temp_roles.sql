-- Table to store roles that should be automatically removed after a duration
CREATE TABLE IF NOT EXISTS temporary_roles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    role_id INTEGER NOT NULL,
    expires_at DATETIME NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Index for quick lookup of expired roles
CREATE INDEX IF NOT EXISTS idx_temp_roles_expiry ON temporary_roles(expires_at);
