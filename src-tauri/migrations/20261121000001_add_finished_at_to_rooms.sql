ALTER TABLE rooms ADD COLUMN finished_at TEXT CHECK (finished_at IS NULL OR finished_at >= created_at);
