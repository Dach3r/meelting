CREATE TABLE repositories (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 80),
    path TEXT NOT NULL CHECK (length(path) > 0),
    git_common_dir TEXT NOT NULL CHECK (length(git_common_dir) > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (workspace_id, git_common_dir),
    UNIQUE (id, workspace_id)
) STRICT;
