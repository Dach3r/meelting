CREATE TABLE task_agents (
    task_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (task_id, agent_id),
    FOREIGN KEY (task_id, workspace_id) REFERENCES tasks (id, workspace_id) ON DELETE CASCADE,
    FOREIGN KEY (agent_id, workspace_id) REFERENCES agents (id, workspace_id) ON DELETE CASCADE
) STRICT;

CREATE INDEX task_agents_agent_id ON task_agents (agent_id);
