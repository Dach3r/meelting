CREATE TABLE task_worktrees (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
    task_id TEXT NOT NULL,
    repository_id TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE CHECK (length(path) > 0),
    branch_name TEXT NOT NULL UNIQUE CHECK (
        length(branch_name) BETWEEN 10 AND 120
        AND substr(branch_name, 1, 9) = 'meelting/'
        AND branch_name NOT GLOB '*[^a-z0-9/-]*'
    ),
    base_ref TEXT NOT NULL CHECK (length(base_ref) > 0),
    base_commit_sha TEXT NOT NULL CHECK (
        length(base_commit_sha) IN (40, 64)
        AND base_commit_sha NOT GLOB '*[^0-9a-f]*'
    ),
    status TEXT NOT NULL CHECK (status IN ('preparing', 'ready', 'failed', 'missing', 'removing', 'removed')),
    error_code TEXT CHECK (
        error_code IS NULL
        OR (length(error_code) BETWEEN 1 AND 64 AND error_code NOT GLOB '*[^a-z_]*')
    ),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    removed_at TEXT,
    CHECK (status <> 'failed' OR error_code IS NOT NULL),
    CHECK (error_code IS NULL OR status IN ('failed', 'removed')),
    CHECK ((status = 'removed') = (removed_at IS NOT NULL)),
    CHECK (updated_at >= created_at),
    CHECK (removed_at IS NULL OR removed_at >= created_at),
    FOREIGN KEY (task_id, repository_id) REFERENCES tasks (id, repository_id) ON DELETE NO ACTION,
    FOREIGN KEY (repository_id, workspace_id) REFERENCES repositories (id, workspace_id) ON DELETE NO ACTION
) STRICT;

CREATE UNIQUE INDEX task_worktrees_active_task_id ON task_worktrees (task_id) WHERE status <> 'removed';
CREATE INDEX task_worktrees_task_id_repository_id ON task_worktrees (task_id, repository_id);
CREATE INDEX task_worktrees_repository_id_workspace_id ON task_worktrees (repository_id, workspace_id);
