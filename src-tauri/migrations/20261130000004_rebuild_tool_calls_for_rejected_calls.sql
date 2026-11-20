CREATE TABLE tool_calls_rebuilt (
    id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL,
    room_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    worktree_id TEXT NOT NULL,
    iteration INTEGER NOT NULL CHECK (iteration >= 1),
    position INTEGER NOT NULL CHECK (position >= 1),
    provider_call_id TEXT NOT NULL CHECK (length(provider_call_id) BETWEEN 1 AND 128),
    tool TEXT CHECK (
        tool IS NULL
        OR (length(tool) BETWEEN 1 AND 64 AND tool NOT GLOB '*[^a-z_]*')
    ),
    arguments TEXT CHECK (
        arguments IS NULL
        OR (
            length(CAST(arguments AS BLOB)) <= 4096
            AND CASE WHEN json_valid(arguments) THEN json_type(arguments) = 'object' ELSE 0 END
        )
    ),
    status TEXT NOT NULL CHECK (status IN ('running', 'succeeded', 'failed', 'cancelled')),
    error_code TEXT CHECK (
        error_code IS NULL
        OR (length(error_code) BETWEEN 1 AND 64 AND error_code NOT GLOB '*[^a-z_]*')
    ),
    result_bytes INTEGER CHECK (result_bytes >= 0),
    truncated INTEGER NOT NULL DEFAULT 0 CHECK (truncated IN (0, 1)),
    started_at TEXT NOT NULL,
    finished_at TEXT,
    CHECK ((status = 'failed') = (error_code IS NOT NULL)),
    CHECK (status <> 'succeeded' OR result_bytes IS NOT NULL),
    CHECK (status <> 'running' OR result_bytes IS NULL),
    CHECK (truncated = 0 OR result_bytes IS NOT NULL),
    CHECK ((status = 'running') = (finished_at IS NULL)),
    CHECK (finished_at IS NULL OR finished_at >= started_at),
    CHECK (
        (tool IS NOT NULL AND arguments IS NOT NULL)
        OR (status = 'failed' AND error_code IS 'invalid_arguments' AND result_bytes IS NULL AND truncated = 0)
    ),
    FOREIGN KEY (run_id, room_id) REFERENCES agent_runs (id, room_id) ON DELETE CASCADE,
    FOREIGN KEY (room_id, workspace_id) REFERENCES rooms (id, workspace_id) ON DELETE CASCADE,
    FOREIGN KEY (task_id, workspace_id) REFERENCES tasks (id, workspace_id) ON DELETE CASCADE,
    FOREIGN KEY (worktree_id, task_id) REFERENCES task_worktrees (id, task_id) ON DELETE CASCADE
) STRICT;

INSERT INTO tool_calls_rebuilt (
    id, run_id, room_id, workspace_id, task_id, worktree_id, iteration, position,
    provider_call_id, tool, arguments, status, error_code, result_bytes, truncated,
    started_at, finished_at
)
SELECT
    id, run_id, room_id, workspace_id, task_id, worktree_id, iteration, position,
    provider_call_id, tool, arguments, status, error_code, result_bytes, truncated,
    started_at, finished_at
FROM tool_calls;

DROP TABLE tool_calls;
ALTER TABLE tool_calls_rebuilt RENAME TO tool_calls;

CREATE UNIQUE INDEX tool_calls_run_id_position ON tool_calls (run_id, position);
CREATE UNIQUE INDEX tool_calls_run_id_provider_call_id ON tool_calls (run_id, provider_call_id);
CREATE INDEX tool_calls_task_id_started_at ON tool_calls (task_id, started_at DESC);
CREATE INDEX tool_calls_worktree_id ON tool_calls (worktree_id);
CREATE INDEX tool_calls_room_id_workspace_id ON tool_calls (room_id, workspace_id);
CREATE INDEX tool_calls_running ON tool_calls (id) WHERE status = 'running';
