ALTER TABLE room_agents ADD COLUMN left_at TEXT CHECK (left_at IS NULL OR left_at >= created_at);
