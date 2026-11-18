CREATE TABLE room_agents (
    room_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (room_id, agent_id),
    FOREIGN KEY (room_id, workspace_id) REFERENCES rooms (id, workspace_id) ON DELETE CASCADE,
    FOREIGN KEY (agent_id, workspace_id) REFERENCES agents (id, workspace_id) ON DELETE CASCADE
) STRICT;

CREATE INDEX room_agents_agent_id ON room_agents (agent_id);
