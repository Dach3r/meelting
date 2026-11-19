ALTER TABLE profile ADD COLUMN worktrees_root TEXT CHECK (worktrees_root IS NULL OR length(worktrees_root) > 0);
