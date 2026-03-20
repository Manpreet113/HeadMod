CREATE TABLE IF NOT EXISTS guild_configs (
    guild_id BIGINT PRIMARY KEY,
    mod_log_channel_id BIGINT,
    message_log_channel_id BIGINT,
    warn_threshold INTEGER NOT NULL DEFAULT 3,
    warn_timeout_secs BIGINT NOT NULL DEFAULT 3600
);

CREATE TABLE IF NOT EXISTS cases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id BIGINT NOT NULL,
    target_id BIGINT NOT NULL,
    moderator_id BIGINT NOT NULL,
    action_type TEXT NOT NULL,
    reason TEXT NOT NULL,
    duration_secs BIGINT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_cases_guild_target ON cases(guild_id, target_id);

CREATE TABLE IF NOT EXISTS scheduled_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id BIGINT NOT NULL,
    target_id BIGINT NOT NULL,
    task_type TEXT NOT NULL,
    execute_at DATETIME NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tasks_execute_at ON scheduled_tasks(execute_at);
