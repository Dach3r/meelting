CREATE TABLE task_room_links (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL,
    room_id TEXT NOT NULL,
    message_id TEXT,
    workspace_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (task_id, room_id, message_id),
    FOREIGN KEY (task_id, workspace_id) REFERENCES tasks (id, workspace_id) ON DELETE CASCADE,
    FOREIGN KEY (room_id, workspace_id) REFERENCES rooms (id, workspace_id) ON DELETE NO ACTION,
    FOREIGN KEY (message_id, room_id) REFERENCES messages (id, room_id) ON DELETE NO ACTION
) STRICT;

CREATE UNIQUE INDEX task_room_links_task_id_room_id_without_message ON task_room_links (task_id, room_id) WHERE message_id IS NULL;
CREATE INDEX task_room_links_room_id ON task_room_links (room_id);
CREATE INDEX task_room_links_message_id ON task_room_links (message_id);
