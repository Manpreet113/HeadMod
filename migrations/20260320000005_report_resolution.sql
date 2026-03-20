-- Add metadata tracking for report resolutions
ALTER TABLE reports ADD COLUMN resolved_at DATETIME;
ALTER TABLE reports ADD COLUMN resolved_by INTEGER;
