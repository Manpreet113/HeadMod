-- Global Intelligence: Shared Ban Network
CREATE TABLE IF NOT EXISTS global_bans (
    user_id INTEGER PRIMARY KEY,
    reason TEXT NOT NULL,
    evidence_url TEXT,
    banned_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    risk_score INTEGER NOT NULL DEFAULT 50 -- 0 to 100
);

-- Risk assessment tracking
CREATE TABLE IF NOT EXISTS user_risk_profile (
    user_id INTEGER PRIMARY KEY,
    total_offenses INTEGER NOT NULL DEFAULT 0,
    last_offense_at DATETIME,
    trust_level INTEGER NOT NULL DEFAULT 100 -- 0 to 100
);
