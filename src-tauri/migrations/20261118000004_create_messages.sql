CREATE TABLE messages (
    id TEXT PRIMARY KEY NOT NULL,
    room_id TEXT NOT NULL REFERENCES rooms (id) ON DELETE CASCADE,
    agent_id TEXT,
    body TEXT NOT NULL CHECK (
        length(trim(body)) > 0
        AND (agent_id IS NOT NULL OR length(body) <= 4000)
    ),
    created_at TEXT NOT NULL,
    FOREIGN KEY (room_id, agent_id) REFERENCES room_agents (room_id, agent_id) ON DELETE NO ACTION
) STRICT;

CREATE INDEX messages_room_id_created_at ON messages (room_id, created_at, id);
CREATE INDEX messages_room_id_agent_id ON messages (room_id, agent_id);
