#!/bin/bash
DATABASE="data.db"
sqlite3 $DATABASE "CREATE TABLE IF NOT EXISTS _sqlx_migrations (version INTEGER PRIMARY KEY, description TEXT NOT NULL, installed_on DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, success BOOLEAN NOT NULL, checksum BLOB NOT NULL);"

versions=(
    "20260319000000:init"
    "20260319000001:automod_updates"
    "20260319000002:audit_logs"
    "20260319000003:automod_caps"
    "20260319000004:alt_detection"
    "20260319000005:tickets_v2"
    "20260319000006:role_persistence"
    "20260320000001:strike_rules"
    "20260320000002:reports"
    "20260320000003:temp_roles"
    "20260320000004:protection_suite"
    "20260320000005:report_resolution"
    "20260320000006:summary_tracking"
)

for item in "${versions[@]}"; do
    v="${item%%:*}"
    d="${item##*:}"
    sqlite3 $DATABASE "INSERT OR IGNORE INTO _sqlx_migrations (version, description, success, checksum) VALUES ($v, '$d', 1, '');"
done

sqlite3 $DATABASE "SELECT version FROM _sqlx_migrations;"
