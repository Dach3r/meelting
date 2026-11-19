ALTER TABLE task_worktrees ADD COLUMN trash_path TEXT CHECK (
    (trash_path IS NULL) = (trashed_at IS NULL)
    AND (trash_path IS NULL OR length(trash_path) > 0)
);

CREATE UNIQUE INDEX task_worktrees_trash_path ON task_worktrees (trash_path) WHERE trash_path IS NOT NULL;
