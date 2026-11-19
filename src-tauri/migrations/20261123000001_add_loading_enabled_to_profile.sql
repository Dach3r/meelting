ALTER TABLE profile ADD COLUMN loading_enabled INTEGER NOT NULL DEFAULT 1 CHECK (loading_enabled IN (0, 1));
