ALTER TABLE profile ADD COLUMN time_format TEXT NOT NULL DEFAULT 'auto' CHECK (time_format IN ('auto', '12h', '24h'));
ALTER TABLE profile ADD COLUMN reduce_motion TEXT NOT NULL DEFAULT 'system' CHECK (reduce_motion IN ('system', 'on', 'off'));
ALTER TABLE profile ADD COLUMN notify_run_succeeded INTEGER NOT NULL DEFAULT 1 CHECK (notify_run_succeeded IN (0, 1));
ALTER TABLE profile ADD COLUMN notify_run_failed INTEGER NOT NULL DEFAULT 1 CHECK (notify_run_failed IN (0, 1));
ALTER TABLE profile ADD COLUMN notify_sound INTEGER NOT NULL DEFAULT 1 CHECK (notify_sound IN (0, 1));
