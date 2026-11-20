# D55 proposal: audit of rejected tool calls

> **Design approved; migration `20261130000004_rebuild_tool_calls_for_rejected_calls.sql` created (2026-11-20) with exactly the SQL below, verified under sqlx in temporary databases only. It has not been applied to any real database; because the app runs its embedded migrations at startup, the first launch of the app with this file present applies it. Applying it outside tests needs explicit authorization.**

## Problem
`tool_calls` (D53) cannot record a call whose arguments are malformed JSON, not an object, over 4096 bytes, or whose tool name is not `[a-z_]{1,64}`: storing it would falsify or persist untrusted data. Stage A rejects such calls before running, answers `error: invalid_arguments` to the model and counts them against the 40-call budget, but leaves no audit row. Until this is fixed the audit is incomplete and tools must stay disabled in production.

## Decision requested (alternative 1, chosen by the product owner)
Allow `tool` and `arguments` to be NULL, **only** on rows that end `failed` with `error_code = 'invalid_arguments'`. Valid calls keep every current constraint.

- A rejected call stores its `provider_call_id`, iteration, position, run, room, workspace, task and environment ids, and times. It never stores the raw arguments or an invalid name.
- `tool` is kept when the name is well formed (so the audit shows which tool was attempted); it is NULL only when the name itself is invalid. `arguments` is NULL for every rejected call whose arguments cannot be stored as a valid object of at most 4096 bytes.
- Calls whose arguments are a valid object but fail the tool's own rules (wrong field, path outside the jail) are not rejected this way: they keep storing their arguments, as today.
- `result_bytes` stays NULL and `truncated` 0 on rejected rows.

## Exact SQL (forward migration `rebuild_tool_calls_for_rejected_calls`)
```sql
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
```
Nothing references `tool_calls`, so the rebuild needs no foreign-key pragma change. The migration runs in sqlx's transaction like `rebuild_agent_runs_for_tool_budgets`.

## Constraint comparison (original `create_tool_calls` vs proposed)
| Constraint | Original | Proposed |
|---|---|---|
| `id` primary key, `NOT NULL` | yes | same |
| `run_id`, `room_id`, `workspace_id`, `task_id`, `worktree_id` `NOT NULL` | yes | same |
| `iteration >= 1`, `position >= 1` | yes | same |
| `provider_call_id` `NOT NULL`, length 1–128 | yes | same |
| `tool` | `NOT NULL`, length 1–64, `[a-z_]` only | nullable; `tool IS NULL OR (length 1–64 AND [a-z_] only)` |
| `arguments` | `NOT NULL`, at most 4096 bytes, valid JSON object | nullable; `arguments IS NULL OR (at most 4096 bytes AND valid JSON object)` |
| `status` in the four states | yes | same |
| `error_code` format | yes | same |
| `result_bytes >= 0`, `truncated` `NOT NULL` in (0, 1) | yes | same |
| `(status = 'failed') = (error_code IS NOT NULL)` | yes | same |
| `status <> 'succeeded' OR result_bytes IS NOT NULL` | yes | same |
| `status <> 'running' OR result_bytes IS NULL` | yes | same |
| `truncated = 0 OR result_bytes IS NOT NULL` | yes | same |
| `(status = 'running') = (finished_at IS NULL)` | yes | same |
| `finished_at IS NULL OR finished_at >= started_at` | yes | same |
| `(tool IS NOT NULL AND arguments IS NOT NULL) OR (status = 'failed' AND error_code IS 'invalid_arguments' AND result_bytes IS NULL AND truncated = 0)` | did not exist (implied by `NOT NULL`) | **new** |
| four composite foreign keys with `ON DELETE CASCADE` | yes | same |
| six indexes (two unique, one partial) | yes | same, recreated |
| `STRICT` | yes | same |

For every row with non-NULL `tool` and `arguments` the proposed constraints accept exactly what the original accepted: the new combined `CHECK` is true by its first alternative. Nothing is relaxed except the `NULL` alternative of the two columns, and that alternative is only reachable through the second alternative of the combined `CHECK`.

## NULL semantics in SQLite `CHECK`
A `CHECK` fails only when its expression is `0`; `NULL` (unknown) **passes**. That is why the proposal does not rely on `NULL` propagating to a failure:
- `tool` / `arguments` column checks start with `IS NULL OR (...)`; for a non-NULL value every operand (`length`, `NOT GLOB`, `json_valid`, `json_type`) returns 0 or 1, never `NULL`.
- The combined check's first alternative uses `IS NOT NULL`, which never returns `NULL`.
- Its second alternative uses `error_code IS 'invalid_arguments'` instead of `=`. With `=`, a `failed` row with `error_code NULL` would evaluate `1 AND NULL AND ...` to `NULL`, and `0 OR NULL` is `NULL`, which would pass; it was only rejected by the older sibling check `(status = 'failed') = (error_code IS NOT NULL)`. `IS` returns 0 or 1, so the combined check is correct on its own (defense in depth). **This is the one change made to the previous SQL.**
- `result_bytes IS NULL` returns 0 or 1; `truncated` is `NOT NULL`.
The matrix in the verification run below exercises each of these cases, including the three-valued-logic one.

## Rebuild procedure (one transaction)
sqlx runs each migration inside a transaction and SQLite DDL is transactional, as in `rebuild_agent_runs_for_tool_budgets`.
1. Create `tool_calls_rebuilt` with the new definition.
2. Copy every row with an explicit column list (`INSERT ... SELECT` from `tool_calls`); any row that violated the new definition would abort the whole migration.
3. `DROP TABLE tool_calls`, `ALTER TABLE tool_calls_rebuilt RENAME TO tool_calls`, recreate the six indexes. Nothing references `tool_calls`, so no `PRAGMA foreign_keys` change is needed; the children-side foreign keys are re-validated on rename.
4. `PRAGMA foreign_key_check` must return no rows (run by the verification, and as a post-migration test).
5. Data verification: row count and a full-row comparison before and after (`SELECT *` ordered by `id`), plus `PRAGMA integrity_check`.

## Rollback
Migrations are forward-only, so rollback is a new migration `restore_tool_calls_strict_columns`, generated from the original `create_tool_calls` definition:
1. `DELETE FROM tool_calls WHERE tool IS NULL OR arguments IS NULL;` (drops only rejected-call audit rows; every other row satisfies the original constraints).
2. Rebuild with the original `CREATE TABLE` (`tool` and `arguments` `NOT NULL`, no combined check), copy, drop, rename, recreate the same six indexes.
The engine code that writes rejected rows must be reverted with it. Exact script (verified below):

```sql
DELETE FROM tool_calls WHERE tool IS NULL OR arguments IS NULL;
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
    tool TEXT NOT NULL CHECK (length(tool) BETWEEN 1 AND 64 AND tool NOT GLOB '*[^a-z_]*'),
    arguments TEXT NOT NULL CHECK (
        length(CAST(arguments AS BLOB)) <= 4096
        AND CASE WHEN json_valid(arguments) THEN json_type(arguments) = 'object' ELSE 0 END
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
    FOREIGN KEY (run_id, room_id) REFERENCES agent_runs (id, room_id) ON DELETE CASCADE,
    FOREIGN KEY (room_id, workspace_id) REFERENCES rooms (id, workspace_id) ON DELETE CASCADE,
    FOREIGN KEY (task_id, workspace_id) REFERENCES tasks (id, workspace_id) ON DELETE CASCADE,
    FOREIGN KEY (worktree_id, task_id) REFERENCES task_worktrees (id, task_id) ON DELETE CASCADE
) STRICT;

INSERT INTO tool_calls_rebuilt (id, run_id, room_id, workspace_id, task_id, worktree_id, iteration, position, provider_call_id, tool, arguments, status, error_code, result_bytes, truncated, started_at, finished_at)
SELECT id, run_id, room_id, workspace_id, task_id, worktree_id, iteration, position, provider_call_id, tool, arguments, status, error_code, result_bytes, truncated, started_at, finished_at FROM tool_calls;

DROP TABLE tool_calls;
ALTER TABLE tool_calls_rebuilt RENAME TO tool_calls;

CREATE UNIQUE INDEX tool_calls_run_id_position ON tool_calls (run_id, position);
CREATE UNIQUE INDEX tool_calls_run_id_provider_call_id ON tool_calls (run_id, provider_call_id);
CREATE INDEX tool_calls_task_id_started_at ON tool_calls (task_id, started_at DESC);
CREATE INDEX tool_calls_worktree_id ON tool_calls (worktree_id);
CREATE INDEX tool_calls_room_id_workspace_id ON tool_calls (room_id, workspace_id);
CREATE INDEX tool_calls_running ON tool_calls (id) WHERE status = 'running';
```

## Verification run (2026-11-20, SQLite CLI 3.51 on a database built from all 32 existing migrations; no migration file was added)
- **Forward:** one transaction, `foreign_key_check` empty, `integrity_check` ok, rows identical before and after (running, succeeded+truncated, failed, cancelled, and rows of two runs), same six indexes, foreign keys still enforced (a bad `worktree_id` is refused), `ON DELETE CASCADE` from a room still removes its calls.
- **NULL matrix, 18 cases, all as expected:** accepted: rejected rows with `tool`/`arguments` NULL in the three combinations, a valid succeeded call, a valid failed call with a tool code, a valid `failed`/`invalid_arguments` call that keeps its arguments. Refused: NULLs with `running`, `succeeded`, `cancelled`, another error code, `failed` with a NULL `error_code`, `invalid_arguments` with `succeeded`, rejected rows with `result_bytes` or `truncated = 1`, an invalid tool name, 4098-byte arguments, array arguments, non-JSON arguments.
- **Failure injection:** an error after `DROP TABLE`, and a row that violates a constraint during the copy, both aborted the transaction with the original table, data and indexes intact and no `tool_calls_rebuilt` left behind.
- **Rollback:** after a rejected row was added, the rollback removed it, kept every other row identical and passed `foreign_key_check`; forward → rollback → forward is repeatable. The only schema text difference after a rebuild is SQLite quoting the table name (`CREATE TABLE "tool_calls"`), which the earlier rebuild migrations also produce.
- **Under sqlx (2026-11-20, temporary databases, sqlx's runner and bundled SQLite):** six tests in `repositories/tool_calls.rs` (`rejected_calls_migration`): the file is byte-identical to the SQL block of this document (the test reads both); the upgrade keeps every row, the six indexes, foreign keys and cascades and passes `foreign_key_check` and `integrity_check`; the 18-case NULL matrix; a failure after the `DROP TABLE` and a failure while copying both leave rows, schema and the sqlx migration record untouched and a later upgrade succeeds; the rollback block of this document drops only rejected rows and restores the original schema, and the upgrade can be applied again.

## Backup and recovery before applying to persistent data (procedure, not automated)
The app applies its embedded migrations on every launch (`database::open`), so the first launch with the D55 file present changes the real database. Nothing in the app takes a backup, and this procedure is documentation only: no backup system was added. The database is `meelting.db` in the app data directory (on macOS `~/Library/Application Support/app.meelting.desktop/`; a `.qa` variant exists). It runs in WAL mode, so recent commits can live only in `meelting.db-wal`.
1. **Back up with SQLite's online backup, not a file copy.** With the app closed (preferred) or open: `sqlite3 "<dir>/meelting.db" ".backup '<dest>/meelting-before-d55.db'"`. It reads through the WAL and returns a consistent snapshot even with live connections. Copying only `meelting.db` loses the changes still in the WAL: in a scratch check a copy of the main file alone had no table at all, while `.backup` returned all 1000 rows, passed `integrity_check` and stayed unchanged while the source kept writing. If files must be copied, close the app first and copy `meelting.db`, `meelting.db-wal` and `meelting.db-shm` together (or run `PRAGMA wal_checkpoint(TRUNCATE)` first).
2. **Verify the backup:** `PRAGMA integrity_check;` returns `ok`, `PRAGMA foreign_key_check;` returns nothing, and `SELECT count(*)` of the main tables (`tool_calls`, `agent_runs`, `messages`, `tasks`, `task_worktrees`) matches the live database. Record a checksum (`shasum -a 256`).
3. **Apply** by launching the app once (only after explicit authorization). Then verify: `PRAGMA integrity_check;`, `PRAGMA foreign_key_check;`, `SELECT version, success FROM _sqlx_migrations ORDER BY version DESC LIMIT 1;` (`20261130000004`, `1`), the same row counts, and that the six `tool_calls` indexes exist.
4. **Recover from a failed or unwanted migration:** a migration failing inside its transaction leaves the database unchanged, so the usual recovery is only to fix the cause and relaunch. To go back to the backup: close the app, move `meelting.db`, `meelting.db-wal` and `meelting.db-shm` aside, copy the verified backup in as `meelting.db` (without the other two), and repeat the checks of step 2.
5. **Downgrade warning:** once D55 is applied, a binary built without it refuses to open the database (sqlx finds an applied migration it does not know). Going back to an older version requires restoring the backup (losing changes made since) or the rollback script above, which also deletes the rejected-call audit rows and must be paired with the matching code.
The procedure was exercised only on scratch databases; it has never been run against user data.

## Application change that goes with it (built 2026-11-20)
- `repositories/tool_calls.rs`: `insert_running` takes optional `tool` and `arguments`; a new `insert_rejected` writes the row already closed `failed` / `invalid_arguments` in one statement.
- `services/tool_loop.rs`: an `Unauditable` call gets a position, is inserted as rejected (name kept only if well formed), still counts against the budget and still answers `error: invalid_arguments`.
- Tools stay disabled in production (`Engine::tools_enabled` is never set) until BR-17 and Windows are closed and the cloud consent exists.

## Review notes (design verification, 2026-11-20)
- **No dangerous argument is stored.** A rejected row keeps no argument text at all (`arguments` NULL) and a tool name only if it already matches `[a-z_]{1,64}`. The remaining model-controlled text in a row is `provider_call_id`; the engine now accepts only `[A-Za-z0-9_.:-]` of 1–128 bytes and treats anything else as `invalid_response` before any insert, so no schema change is needed for it.
- **Valid calls are not weakened.** Valid rows still need non-NULL `tool` and `arguments`, the 4096-byte object check and the name format; the new alternative only matches `failed` + `invalid_arguments` with no result, so a `running`, `succeeded` or `cancelled` row can never have NULLs.
- **Not covered by the schema, by design:** a call that is a valid JSON object but fails the tool's own rules keeps its arguments (they are already inside the 4096-byte object limit) and is closed `failed` with the tool's error code, as today.
- **Ambiguity to confirm:** the engine, not the schema, decides which calls are "rejected"; a database-only reader sees `invalid_arguments` rows with NULLs and cannot tell malformed JSON from an oversized one. If that distinction matters, a second code (`arguments_too_large`) would need a separate approval.

## Tests required (to write with the migration, after approval)
- Rust schema tests in `repositories/tool_calls.rs` repeating the 18-case NULL matrix against the sqlx-migrated database, and that `error_code IS 'invalid_arguments'` rejects the `failed` + NULL `error_code` case on its own (the sibling check dropped in a scratch copy).
- Migration on data under sqlx: rows of every status survive, `foreign_key_check` is empty, cascades from run, room, task and environment still work.
- Failure during migration under sqlx (a forced error after the drop) leaves the original table, indexes and data intact.
- Rollback script: removes only rejected rows and restores the original constraints.
- Schema matrix: a rejected row is accepted only with `failed` + `invalid_arguments` and NULL `result_bytes`; NULL `tool` or `arguments` is refused for `running`, `succeeded`, `cancelled` and for other error codes; valid rows keep the 4096-byte, JSON-object and name-format checks.
- Migration on data: existing rows (all statuses, truncated flags) survive unchanged; unique indexes, composite foreign keys and cascades from run, room, task and environment still work.
- Engine: malformed JSON, array, scalar, 4097-byte arguments and an invalid name each produce exactly one rejected row, no raw text anywhere in the table, budget spent, model answered; a well-formed name is kept; the position sequence has no gaps; cancellation and startup recovery are unaffected.
- Privacy: a search over the whole `tool_calls` table after those tests finds none of the rejected payloads.
