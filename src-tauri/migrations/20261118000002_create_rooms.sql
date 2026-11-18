CREATE TABLE rooms (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 80),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (id, workspace_id)
) STRICT;

CREATE INDEX rooms_workspace_id_updated_at ON rooms (workspace_id, updated_at DESC, id DESC);
