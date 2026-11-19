CREATE TABLE tasks (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
    repository_id TEXT,
    origin_room_id TEXT,
    origin_message_id TEXT,
    title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 120),
    description TEXT NOT NULL CHECK (length(description) <= 10000),
    status TEXT NOT NULL CHECK (status IN ('pending', 'in_progress', 'blocked', 'completed', 'canceled')),
    priority TEXT NOT NULL CHECK (priority IN ('low', 'medium', 'high')),
    base_ref TEXT CHECK (base_ref IS NULL OR length(base_ref) > 0),
    completed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (id, workspace_id),
    CHECK ((status = 'completed') = (completed_at IS NOT NULL)),
    CHECK (base_ref IS NULL OR repository_id IS NOT NULL),
    CHECK (origin_message_id IS NULL OR origin_room_id IS NOT NULL),
    FOREIGN KEY (repository_id, workspace_id) REFERENCES repositories (id, workspace_id) ON DELETE NO ACTION,
    FOREIGN KEY (origin_room_id, workspace_id) REFERENCES rooms (id, workspace_id) ON DELETE NO ACTION,
    FOREIGN KEY (origin_message_id, origin_room_id) REFERENCES messages (id, room_id) ON DELETE NO ACTION
) STRICT;

CREATE INDEX tasks_workspace_id_updated_at ON tasks (workspace_id, updated_at DESC, id DESC);
CREATE INDEX tasks_repository_id ON tasks (repository_id);
CREATE INDEX tasks_origin_room_id ON tasks (origin_room_id);
CREATE INDEX tasks_origin_message_id ON tasks (origin_message_id);
