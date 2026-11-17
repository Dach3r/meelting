CREATE TABLE profile (
    id TEXT PRIMARY KEY NOT NULL CHECK (id = 'singleton'),
    locale TEXT NOT NULL CHECK (locale IN ('es', 'en')),
    current_workspace_id TEXT REFERENCES workspaces (id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;
