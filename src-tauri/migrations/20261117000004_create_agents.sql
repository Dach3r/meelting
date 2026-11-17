CREATE TABLE agents (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
    team_id TEXT NOT NULL,
    template_key TEXT,
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 80),
    role TEXT NOT NULL CHECK (length(trim(role)) BETWEEN 1 AND 80),
    description TEXT NOT NULL CHECK (length(description) <= 500),
    instructions TEXT NOT NULL CHECK (length(instructions) <= 10000),
    provider TEXT NOT NULL CHECK (length(trim(provider)) > 0),
    model TEXT NOT NULL CHECK (length(trim(model)) > 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (team_id, workspace_id) REFERENCES teams (id, workspace_id) ON DELETE RESTRICT
) STRICT;

CREATE INDEX agents_workspace_id ON agents (workspace_id);
CREATE INDEX agents_team_id ON agents (team_id);
