CREATE TABLE canvas_positions (
    workspace_id TEXT NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
    node_id TEXT NOT NULL,
    x REAL NOT NULL,
    y REAL NOT NULL,
    PRIMARY KEY (workspace_id, node_id)
) STRICT;
