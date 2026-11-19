CREATE TABLE task_worktree_file_copies (
    task_worktree_id TEXT NOT NULL REFERENCES task_worktrees (id) ON DELETE CASCADE,
    relative_path TEXT NOT NULL CHECK (
        length(relative_path) BETWEEN 1 AND 1024
        AND substr(relative_path, 1, 1) <> '/'
        AND instr(relative_path, '\') = 0
        AND instr(relative_path, ':') = 0
        AND '/' || relative_path || '/' NOT LIKE '%/../%'
        AND '/' || relative_path || '/' NOT LIKE '%/./%'
        AND '/' || relative_path || '/' NOT LIKE '%//%'
        AND '/' || relative_path || '/' NOT LIKE '%/.git/%'
    ),
    size_bytes INTEGER NOT NULL CHECK (size_bytes BETWEEN 0 AND 10485760),
    copied_at TEXT NOT NULL,
    PRIMARY KEY (task_worktree_id, relative_path)
) STRICT;
