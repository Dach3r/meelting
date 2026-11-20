CREATE TABLE agent_runs_rebuilt (
    id TEXT PRIMARY KEY NOT NULL,
    room_id TEXT NOT NULL REFERENCES rooms (id) ON DELETE CASCADE,
    agent_id TEXT NOT NULL,
    request_message_id TEXT NOT NULL,
    response_message_id TEXT UNIQUE,
    client_request_id TEXT NOT NULL CHECK (
        length(client_request_id) = 36 AND client_request_id = lower(client_request_id)
    ),
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'passed', 'failed')),
    error_kind TEXT CHECK (error_kind IN (
        'rate_limited', 'timeout', 'provider_unavailable',
        'provider_not_configured', 'model_unavailable', 'provider_auth_failed',
        'provider_billing', 'provider_rejected', 'content_filtered',
        'output_truncated', 'invalid_response', 'context_too_large',
        'cancelled', 'expired',
        'tools_unsupported', 'iteration_budget_exhausted',
        'tool_call_budget_exhausted', 'tool_output_budget_exhausted'
    )),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 3),
    provider TEXT NOT NULL CHECK (length(trim(provider)) > 0),
    model TEXT NOT NULL CHECK (length(trim(model)) > 0),
    input_tokens INTEGER CHECK (input_tokens >= 0),
    output_tokens INTEGER CHECK (output_tokens >= 0),
    created_at TEXT NOT NULL,
    started_at TEXT,
    finished_at TEXT,
    CHECK ((status = 'failed') = (error_kind IS NOT NULL)),
    CHECK ((status = 'succeeded') = (response_message_id IS NOT NULL)),
    CHECK ((status IN ('succeeded', 'passed', 'failed')) = (finished_at IS NOT NULL)),
    CHECK ((attempts > 0) = (started_at IS NOT NULL)),
    CHECK (status NOT IN ('running', 'succeeded', 'passed') OR attempts > 0),
    CHECK (status <> 'queued' OR attempts < 3),
    CHECK (started_at IS NULL OR started_at >= created_at),
    CHECK (finished_at IS NULL OR finished_at >= created_at),
    CHECK (finished_at IS NULL OR started_at IS NULL OR finished_at >= started_at),
    CHECK (response_message_id IS NULL OR response_message_id <> request_message_id),
    FOREIGN KEY (room_id, agent_id) REFERENCES room_agents (room_id, agent_id) ON DELETE NO ACTION,
    FOREIGN KEY (request_message_id, room_id) REFERENCES messages (id, room_id) ON DELETE NO ACTION,
    FOREIGN KEY (response_message_id, room_id, agent_id) REFERENCES messages (id, room_id, agent_id) ON DELETE NO ACTION
) STRICT;

INSERT INTO agent_runs_rebuilt (
    id, room_id, agent_id, request_message_id, response_message_id, client_request_id,
    status, error_kind, attempts, provider, model, input_tokens, output_tokens,
    created_at, started_at, finished_at
)
SELECT
    id, room_id, agent_id, request_message_id, response_message_id, client_request_id,
    status, error_kind, attempts, provider, model, input_tokens, output_tokens,
    created_at, started_at, finished_at
FROM agent_runs;

DROP TABLE agent_runs;
ALTER TABLE agent_runs_rebuilt RENAME TO agent_runs;

CREATE UNIQUE INDEX agent_runs_request_message_id_agent_id ON agent_runs (request_message_id, agent_id);
CREATE UNIQUE INDEX agent_runs_room_id_client_request_id_agent_id ON agent_runs (room_id, client_request_id, agent_id);
CREATE UNIQUE INDEX agent_runs_active_room_id_agent_id ON agent_runs (room_id, agent_id)
    WHERE status IN ('queued', 'running');
CREATE INDEX agent_runs_room_id_agent_id ON agent_runs (room_id, agent_id);
CREATE UNIQUE INDEX agent_runs_id_room_id ON agent_runs (id, room_id);
