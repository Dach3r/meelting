use sqlx::SqliteExecutor;

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";

pub struct NewCall<'a> {
    pub id: &'a str,
    pub run_id: &'a str,
    pub room_id: &'a str,
    pub workspace_id: &'a str,
    pub task_id: &'a str,
    pub worktree_id: &'a str,
    pub iteration: i64,
    pub position: i64,
    pub provider_call_id: &'a str,
    pub tool: &'a str,
    pub arguments: &'a str,
}

pub async fn insert_running<'e>(
    executor: impl SqliteExecutor<'e>,
    call: &NewCall<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO tool_calls (id, run_id, room_id, workspace_id, task_id, worktree_id, iteration, \
         position, provider_call_id, tool, arguments, status, started_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'running', {NOW})"
    )))
    .bind(call.id)
    .bind(call.run_id)
    .bind(call.room_id)
    .bind(call.workspace_id)
    .bind(call.task_id)
    .bind(call.worktree_id)
    .bind(call.iteration)
    .bind(call.position)
    .bind(call.provider_call_id)
    .bind(call.tool)
    .bind(call.arguments)
    .execute(executor)
    .await?;
    Ok(())
}

pub struct NewRejected<'a> {
    pub id: &'a str,
    pub run_id: &'a str,
    pub room_id: &'a str,
    pub workspace_id: &'a str,
    pub task_id: &'a str,
    pub worktree_id: &'a str,
    pub iteration: i64,
    pub position: i64,
    pub provider_call_id: &'a str,
    pub tool: Option<&'a str>,
}

pub async fn insert_rejected<'e>(
    executor: impl SqliteExecutor<'e>,
    call: &NewRejected<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO tool_calls (id, run_id, room_id, workspace_id, task_id, worktree_id, iteration, \
         position, provider_call_id, tool, arguments, status, error_code, started_at, finished_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, 'failed', 'invalid_arguments', {NOW}, {NOW})"
    )))
    .bind(call.id)
    .bind(call.run_id)
    .bind(call.room_id)
    .bind(call.workspace_id)
    .bind(call.task_id)
    .bind(call.worktree_id)
    .bind(call.iteration)
    .bind(call.position)
    .bind(call.provider_call_id)
    .bind(call.tool)
    .execute(executor)
    .await?;
    Ok(())
}

pub enum Closing<'a> {
    Succeeded { result_bytes: i64, truncated: bool },
    Failed { code: &'a str },
    Cancelled,
}

pub async fn close<'e>(
    executor: impl SqliteExecutor<'e>,
    id: &str,
    closing: &Closing<'_>,
) -> Result<bool, sqlx::Error> {
    let (status, code, result_bytes, truncated) = match closing {
        Closing::Succeeded {
            result_bytes,
            truncated,
        } => ("succeeded", None, Some(*result_bytes), *truncated),
        Closing::Failed { code } => ("failed", Some(*code), None, false),
        Closing::Cancelled => ("cancelled", None, None, false),
    };
    let result = sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE tool_calls SET status = ?, error_code = ?, result_bytes = ?, truncated = ?, \
         finished_at = MAX({NOW}, started_at) WHERE id = ? AND status = 'running'"
    )))
    .bind(status)
    .bind(code)
    .bind(result_bytes)
    .bind(truncated)
    .bind(id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn cancel_all_running<'e>(executor: impl SqliteExecutor<'e>) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE tool_calls SET status = 'cancelled', finished_at = MAX({NOW}, started_at) \
         WHERE status = 'running'"
    )))
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::database::open_for_test;
    use sqlx::SqlitePool;

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
    const START: &str = "2026-11-19T10:00:00.000Z";
    const END: &str = "2026-11-19T10:00:01.000Z";

    const COLUMNS: [&str; 17] = [
        "id",
        "run_id",
        "room_id",
        "workspace_id",
        "task_id",
        "worktree_id",
        "iteration",
        "position",
        "provider_call_id",
        "tool",
        "arguments",
        "status",
        "error_code",
        "result_bytes",
        "truncated",
        "started_at",
        "finished_at",
    ];

    async fn exec(pool: &SqlitePool, sql: &str) -> Result<u64, sqlx::Error> {
        sqlx::query(sqlx::AssertSqlSafe(sql.to_string()))
            .execute(pool)
            .await
            .map(|result| result.rows_affected())
    }

    async fn count(pool: &SqlitePool, table: &str) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
            .fetch_one(pool)
            .await
            .unwrap()
    }

    fn worktree(id: &str, task: &str, workspace: &str) -> String {
        format!(
            "INSERT INTO task_worktrees (id, workspace_id, task_id, repository_id, path, branch_name, base_ref, base_commit_sha, status, created_at, updated_at) \
             VALUES ('{id}', '{workspace}', '{task}', 'p1', '/wt/{id}', 'meelting/fix-{id}', 'main', '{SHA}', 'ready', 'x', 'x')"
        )
    }

    async fn seeded() -> (SqlitePool, tempfile::TempDir) {
        let (pool, dir) = open_for_test().await;
        seed(&pool).await;
        (pool, dir)
    }

    async fn seed(pool: &SqlitePool) {
        for sql in [
            "INSERT INTO workspaces VALUES ('w1', 'A', 'x', 'x'), ('w2', 'B', 'x', 'x')",
            "INSERT INTO teams VALUES ('g1', 'w1', 'General', 'x', 'x'), ('g2', 'w2', 'Other', 'x', 'x')",
            "INSERT INTO agents (id, workspace_id, team_id, name, role, description, instructions, provider, model, created_at, updated_at) \
             VALUES ('a1', 'w1', 'g1', 'Ada', 'Dev', '', '', 'local', 'qwen', 'x', 'x'), \
                    ('a2', 'w2', 'g2', 'Bob', 'Dev', '', '', 'local', 'qwen', 'x', 'x')",
            "INSERT INTO rooms (id, workspace_id, name, created_at, updated_at) VALUES ('r1', 'w1', 'Daily', 'x', 'x'), ('r2', 'w1', 'Other', 'x', 'x'), ('r3', 'w2', 'Far', 'x', 'x')",
            "INSERT INTO room_agents (room_id, agent_id, workspace_id, created_at) VALUES ('r1', 'a1', 'w1', 'x'), ('r2', 'a1', 'w1', 'x'), ('r3', 'a2', 'w2', 'x')",
            "INSERT INTO messages (id, room_id, agent_id, body, created_at) VALUES ('m1', 'r1', NULL, 'Hi', 'x'), ('m2', 'r2', NULL, 'Hi', 'x')",
            "INSERT INTO repositories VALUES ('p1', 'w1', 'app', '/app', '/app/.git', 'x', 'x'), ('p2', 'w2', 'app', '/app', '/app/.git', 'x', 'x')",
            "INSERT INTO tasks (id, workspace_id, title, description, status, priority, repository_id, created_at, updated_at) \
             VALUES ('t1', 'w1', 'Fix', '', 'pending', 'low', 'p1', 'x', 'x'), ('t2', 'w1', 'Fix', '', 'pending', 'low', 'p1', 'x', 'x')",
            "INSERT INTO tasks (id, workspace_id, title, description, status, priority, repository_id, created_at, updated_at) \
             VALUES ('t3', 'w2', 'Fix', '', 'pending', 'low', 'p2', 'x', 'x')",
            "INSERT INTO agent_runs (id, room_id, agent_id, request_message_id, client_request_id, status, attempts, provider, model, created_at) \
             VALUES ('run1', 'r1', 'a1', 'm1', '0196f3a0-0000-7000-8000-000000000001', 'queued', 0, 'local', 'qwen', 'x'), \
                    ('run2', 'r2', 'a1', 'm2', '0196f3a0-0000-7000-8000-000000000002', 'queued', 0, 'local', 'qwen', 'x')",
        ] {
            exec(pool, sql).await.unwrap();
        }
        exec(pool, &worktree("k1", "t1", "w1")).await.unwrap();
        exec(pool, &worktree("k2", "t2", "w1")).await.unwrap();
    }

    fn default_value(column: &str) -> String {
        match column {
            "id" => "'c1'",
            "run_id" => "'run1'",
            "room_id" => "'r1'",
            "workspace_id" => "'w1'",
            "task_id" => "'t1'",
            "worktree_id" => "'k1'",
            "iteration" => "1",
            "position" => "1",
            "provider_call_id" => "'call_1'",
            "tool" => "'read_file'",
            "arguments" => "'{\"path\":\"src/lib.rs\"}'",
            "status" => "'running'",
            "truncated" => "0",
            "started_at" => return format!("'{START}'"),
            _ => "NULL",
        }
        .to_string()
    }

    fn call(overrides: &[(&str, &str)]) -> String {
        let values: Vec<String> = COLUMNS
            .iter()
            .map(|column| {
                overrides
                    .iter()
                    .rfind(|(name, _)| name == column)
                    .map(|(_, value)| value.to_string())
                    .unwrap_or_else(|| default_value(column))
            })
            .collect();
        format!(
            "INSERT INTO tool_calls ({}) VALUES ({})",
            COLUMNS.join(", "),
            values.join(", ")
        )
    }

    fn finished(status: &str) -> Vec<(&'static str, String)> {
        let status = format!("'{status}'");
        vec![("status", status), ("finished_at", format!("'{END}'"))]
    }

    fn borrowed<'a>(pairs: &'a [(&'static str, String)]) -> Vec<(&'a str, &'a str)> {
        pairs.iter().map(|(k, v)| (*k, v.as_str())).collect()
    }

    #[tokio::test]
    async fn a_call_must_share_run_room_workspace_task_and_worktree() {
        let (pool, _dir) = seeded().await;

        for (index, sql) in [
            call(&[("run_id", "'run2'")]),
            call(&[("room_id", "'r3'"), ("workspace_id", "'w2'")]),
            call(&[("room_id", "'r2'")]),
            call(&[("workspace_id", "'w2'")]),
            call(&[("task_id", "'t3'")]),
            call(&[("task_id", "'t2'")]),
            call(&[("worktree_id", "'k2'")]),
            call(&[("worktree_id", "'missing'")]),
            call(&[("run_id", "'missing'")]),
        ]
        .iter()
        .enumerate()
        {
            assert!(exec(&pool, sql).await.is_err(), "case {index} was accepted");
        }
        exec(&pool, &call(&[])).await.unwrap();
    }

    #[tokio::test]
    async fn arguments_are_a_json_object_of_at_most_4096_utf8_bytes() {
        let (pool, _dir) = seeded().await;
        let spaced = |bytes: usize| format!("'{{\"p\":\"{}\"}}'", "a".repeat(bytes - 8));
        let accents = |chars: usize| format!("'{{\"p\":\"{}\"}}'", "é".repeat(chars));

        for (index, arguments) in [
            "'not json'".to_string(),
            "'[]'".to_string(),
            "'\"text\"'".to_string(),
            "'null'".to_string(),
            spaced(4097),
            accents(2045),
        ]
        .iter()
        .enumerate()
        {
            let sql = call(&[("arguments", arguments)]);
            assert!(
                exec(&pool, &sql).await.is_err(),
                "case {index} was accepted"
            );
        }
        exec(&pool, &call(&[("arguments", &spaced(4096))]))
            .await
            .unwrap();
        exec(
            &pool,
            &call(&[
                ("id", "'c2'"),
                ("position", "2"),
                ("provider_call_id", "'call_2'"),
                ("arguments", &accents(2044)),
            ]),
        )
        .await
        .unwrap();
        exec(
            &pool,
            &call(&[
                ("id", "'c3'"),
                ("position", "3"),
                ("provider_call_id", "'call_3'"),
                ("arguments", "'{}'"),
            ]),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn status_error_result_and_truncation_follow_the_state_machine() {
        let (pool, _dir) = seeded().await;
        let succeeded = finished("succeeded");
        let failed = finished("failed");
        let cancelled = finished("cancelled");

        let rejected: Vec<Vec<(&str, &str)>> = vec![
            vec![("status", "'done'")],
            vec![
                ("status", "'running'"),
                ("finished_at", "'2026-11-19T10:00:01.000Z'"),
            ],
            vec![("status", "'running'"), ("result_bytes", "10")],
            vec![("status", "'running'"), ("error_code", "'not_found'")],
            [borrowed(&succeeded), vec![("result_bytes", "NULL")]].concat(),
            [
                borrowed(&succeeded),
                vec![("result_bytes", "10"), ("error_code", "'x'")],
            ]
            .concat(),
            [borrowed(&failed), vec![("error_code", "NULL")]].concat(),
            [borrowed(&failed), vec![("error_code", "'Not Found'")]].concat(),
            [borrowed(&cancelled), vec![("error_code", "'cancelled'")]].concat(),
            [borrowed(&succeeded), vec![("result_bytes", "-1")]].concat(),
            [
                borrowed(&succeeded),
                vec![("result_bytes", "10"), ("truncated", "2")],
            ]
            .concat(),
            vec![("truncated", "1")],
            [borrowed(&cancelled), vec![("truncated", "1")]].concat(),
            [
                borrowed(&succeeded),
                vec![
                    ("result_bytes", "10"),
                    ("finished_at", "'2026-11-19T09:59:59.000Z'"),
                ],
            ]
            .concat(),
            vec![("iteration", "0")],
            vec![("position", "0")],
            vec![("tool", "'Run Command'")],
            vec![("provider_call_id", "''")],
        ];
        for (index, overrides) in rejected.iter().enumerate() {
            assert!(
                exec(&pool, &call(overrides)).await.is_err(),
                "case {index} was accepted"
            );
        }

        let accepted: Vec<Vec<(&str, &str)>> = vec![
            vec![],
            [borrowed(&succeeded), vec![("result_bytes", "10")]].concat(),
            [
                borrowed(&succeeded),
                vec![("result_bytes", "16384"), ("truncated", "1")],
            ]
            .concat(),
            [borrowed(&failed), vec![("error_code", "'not_found'")]].concat(),
            [
                borrowed(&failed),
                vec![
                    ("error_code", "'timeout'"),
                    ("result_bytes", "32768"),
                    ("truncated", "1"),
                ],
            ]
            .concat(),
            borrowed(&cancelled),
            [
                borrowed(&cancelled),
                vec![("result_bytes", "5"), ("truncated", "1")],
            ]
            .concat(),
        ];
        for (index, overrides) in accepted.iter().enumerate() {
            let position = (index + 1).to_string();
            let id = format!("'c{position}'");
            let provider_call_id = format!("'call_{position}'");
            let mut all = vec![
                ("id", id.as_str()),
                ("position", position.as_str()),
                ("provider_call_id", provider_call_id.as_str()),
            ];
            all.extend(overrides.iter().copied());
            exec(&pool, &call(&all))
                .await
                .unwrap_or_else(|error| panic!("case {index} was rejected: {error}"));
        }
    }

    #[tokio::test]
    async fn position_and_provider_call_id_are_unique_per_run() {
        let (pool, _dir) = seeded().await;
        exec(&pool, &call(&[])).await.unwrap();

        assert!(
            exec(
                &pool,
                &call(&[("id", "'c2'"), ("provider_call_id", "'call_2'")])
            )
            .await
            .is_err()
        );
        assert!(
            exec(&pool, &call(&[("id", "'c3'"), ("position", "2")]))
                .await
                .is_err()
        );
        exec(
            &pool,
            &call(&[("id", "'c4'"), ("run_id", "'run2'"), ("room_id", "'r2'")]),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn calls_follow_the_lifecycle_of_the_run_the_room_the_task_and_the_environment() {
        for (statements, remaining) in [
            (vec!["DELETE FROM agent_runs WHERE id = 'run1'"], 0),
            (vec!["DELETE FROM rooms WHERE id = 'r1'"], 0),
            (vec!["DELETE FROM task_worktrees WHERE id = 'k1'"], 0),
            (
                vec![
                    "DELETE FROM task_worktrees WHERE task_id = 't1'",
                    "DELETE FROM tasks WHERE id = 't1'",
                ],
                0,
            ),
            (
                vec![
                    "DELETE FROM task_worktrees WHERE task_id = 't2'",
                    "DELETE FROM tasks WHERE id = 't2'",
                ],
                1,
            ),
        ] {
            let (pool, _dir) = seeded().await;
            exec(&pool, &call(&[])).await.unwrap();

            for statement in &statements {
                exec(&pool, statement).await.unwrap();
            }

            assert_eq!(
                count(&pool, "tool_calls").await,
                remaining,
                "{statements:?}"
            );
            let violations = sqlx::query("SELECT 1 FROM pragma_foreign_key_check")
                .fetch_all(&pool)
                .await
                .unwrap();
            assert!(violations.is_empty(), "{statements:?}");
        }
    }

    #[tokio::test]
    async fn the_calls_still_running_are_found_through_their_own_index() {
        let (pool, _dir) = seeded().await;
        let rows: Vec<(i64, i64, i64, String)> =
            sqlx::query_as("EXPLAIN QUERY PLAN SELECT id FROM tool_calls WHERE status = 'running'")
                .fetch_all(&pool)
                .await
                .unwrap();

        assert!(
            rows.iter().any(|row| row.3.contains("tool_calls_running")),
            "{rows:?}"
        );
    }

    fn new_call<'a>(id: &'a str, position: i64, arguments: &'a str) -> NewCall<'a> {
        NewCall {
            id,
            run_id: "run1",
            room_id: "r1",
            workspace_id: "w1",
            task_id: "t1",
            worktree_id: "k1",
            iteration: 1,
            position,
            provider_call_id: id,
            tool: "list_dir",
            arguments,
        }
    }

    async fn status_of(pool: &SqlitePool, id: &str) -> (String, Option<String>, Option<i64>, i64) {
        sqlx::query_as(
            "SELECT status, error_code, result_bytes, truncated FROM tool_calls WHERE id = ?",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn a_call_is_recorded_running_and_closed_exactly_once() {
        let (pool, _dir) = seeded().await;
        insert_running(&pool, &new_call("c1", 1, "{\"path\":\"src\"}"))
            .await
            .unwrap();
        assert_eq!(status_of(&pool, "c1").await.0, "running");

        let closing = Closing::Succeeded {
            result_bytes: 12,
            truncated: true,
        };
        assert!(close(&pool, "c1", &closing).await.unwrap());
        assert!(!close(&pool, "c1", &Closing::Cancelled).await.unwrap());
        assert_eq!(
            status_of(&pool, "c1").await,
            ("succeeded".into(), None, Some(12), 1)
        );
    }

    #[tokio::test]
    async fn failures_keep_their_code_and_no_result() {
        let (pool, _dir) = seeded().await;
        insert_running(&pool, &new_call("c1", 1, "{}"))
            .await
            .unwrap();

        close(
            &pool,
            "c1",
            &Closing::Failed {
                code: "path_outside",
            },
        )
        .await
        .unwrap();

        assert_eq!(
            status_of(&pool, "c1").await,
            ("failed".into(), Some("path_outside".into()), None, 0)
        );
    }

    #[tokio::test]
    async fn the_arguments_that_cannot_be_stored_faithfully_are_refused_by_the_schema() {
        let (pool, _dir) = seeded().await;

        for arguments in ["not json", "[]", "\"x\""] {
            assert!(
                insert_running(&pool, &new_call("c1", 1, arguments))
                    .await
                    .is_err(),
                "{arguments}"
            );
        }
        assert_eq!(count(&pool, "tool_calls").await, 0);
    }

    #[tokio::test]
    async fn recovery_cancels_only_the_running_calls_and_is_idempotent() {
        let (pool, _dir) = seeded().await;
        insert_running(&pool, &new_call("c1", 1, "{}"))
            .await
            .unwrap();
        insert_running(&pool, &new_call("c2", 2, "{}"))
            .await
            .unwrap();
        close(
            &pool,
            "c2",
            &Closing::Succeeded {
                result_bytes: 1,
                truncated: false,
            },
        )
        .await
        .unwrap();
        let other = NewCall {
            run_id: "run2",
            room_id: "r2",
            ..new_call("c3", 1, "{}")
        };
        insert_running(&pool, &other).await.unwrap();

        assert_eq!(cancel_all_running(&pool).await.unwrap(), 2);
        assert_eq!(cancel_all_running(&pool).await.unwrap(), 0);
        assert_eq!(status_of(&pool, "c1").await.0, "cancelled");
        assert_eq!(status_of(&pool, "c2").await.0, "succeeded");
        assert_eq!(status_of(&pool, "c3").await.0, "cancelled");
    }

    mod rejected_calls_migration {
        use super::*;
        use sqlx::migrate::{Migration, MigrationType, Migrator};
        use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
        use sqlx::{AssertSqlSafe, SqlSafeStr};
        use std::borrow::Cow;

        const DOC: &str = include_str!("../../../docs/product/d55-tool-call-audit-proposal.md");
        const MIGRATION: &str = include_str!(
            "../../migrations/20261130000004_rebuild_tool_calls_for_rejected_calls.sql"
        );
        const VERSION: i64 = 20261130000004;
        const LATER: &str = "'2026-11-19T10:00:01.000Z'";

        fn block(prefix: &str) -> String {
            DOC.replace("\r\n", "\n")
                .split("```sql\n")
                .skip(1)
                .filter_map(|section| section.split("```").next().map(str::to_string))
                .find(|sql| sql.starts_with(prefix))
                .unwrap()
        }

        fn rollback_sql() -> String {
            block("DELETE FROM tool_calls")
        }

        async fn pool_before_the_migration() -> (SqlitePool, tempfile::TempDir) {
            let dir = tempfile::tempdir().unwrap();
            let options = SqliteConnectOptions::new()
                .filename(dir.path().join("migration.db"))
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .foreign_keys(true);
            let pool = SqlitePoolOptions::new()
                .connect_with(options)
                .await
                .unwrap();
            let all = sqlx::migrate!("./migrations");
            let older: Vec<Migration> = all
                .migrations
                .iter()
                .filter(|migration| migration.version < VERSION)
                .cloned()
                .collect();
            assert_eq!(older.len() + 1, all.migrations.len());
            Migrator {
                migrations: Cow::Owned(older),
                ..sqlx::migrate!("./migrations")
            }
            .run(&pool)
            .await
            .unwrap();
            (pool, dir)
        }

        async fn upgrade(pool: &SqlitePool) {
            sqlx::migrate!("./migrations").run(pool).await.unwrap();
        }

        async fn rows(pool: &SqlitePool) -> Vec<String> {
            let quoted: Vec<String> = COLUMNS
                .iter()
                .map(|column| format!("quote({column})"))
                .collect();
            sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT {} FROM tool_calls ORDER BY id",
                quoted.join(" || '|' || ")
            )))
            .fetch_all(pool)
            .await
            .unwrap()
        }

        async fn schema(pool: &SqlitePool) -> String {
            let text: String = sqlx::query_scalar(
                "SELECT group_concat(name || ':' || ifnull(sql, ''), char(10)) \
                 FROM (SELECT name, sql FROM sqlite_master WHERE tbl_name = 'tool_calls' ORDER BY name)",
            )
            .fetch_one(pool)
            .await
            .unwrap();
            text.replace('"', "")
        }

        async fn pragma(pool: &SqlitePool, statement: &'static str) -> Vec<String> {
            sqlx::query_scalar(statement)
                .fetch_all(pool)
                .await
                .unwrap_or_default()
        }

        async fn existing_rows(pool: &SqlitePool) {
            for (id, position, overrides) in [
                ("c1", 1, vec![]),
                (
                    "c2",
                    2,
                    vec![
                        ("result_bytes", "12"),
                        ("truncated", "1"),
                        ("status", "'succeeded'"),
                        ("finished_at", LATER),
                    ],
                ),
                (
                    "c3",
                    3,
                    vec![
                        ("error_code", "'path_outside'"),
                        ("status", "'failed'"),
                        ("finished_at", LATER),
                    ],
                ),
                (
                    "c4",
                    4,
                    vec![("status", "'cancelled'"), ("finished_at", LATER)],
                ),
            ] {
                let id = format!("'{id}'");
                let position = position.to_string();
                let provider = format!("'call_{position}'");
                let mut all = vec![
                    ("id", id.as_str()),
                    ("position", position.as_str()),
                    ("provider_call_id", provider.as_str()),
                ];
                all.extend(overrides);
                exec(pool, &call(&all)).await.unwrap();
            }
            exec(
                pool,
                &call(&[
                    ("id", "'other'"),
                    ("run_id", "'run2'"),
                    ("room_id", "'r2'"),
                    ("provider_call_id", "'call_x'"),
                ]),
            )
            .await
            .unwrap();
        }

        #[test]
        fn the_migration_is_exactly_the_sql_approved_in_the_proposal() {
            assert_eq!(
                MIGRATION.trim(),
                block("CREATE TABLE tool_calls_rebuilt").trim()
            );
        }

        #[tokio::test]
        async fn the_upgrade_keeps_every_row_index_foreign_key_and_cascade() {
            let (pool, _dir) = pool_before_the_migration().await;
            seed(&pool).await;
            existing_rows(&pool).await;
            let before = rows(&pool).await;
            let indexes = pragma(&pool, "SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'tool_calls' AND name NOT LIKE 'sqlite_%' ORDER BY name").await;

            upgrade(&pool).await;

            assert_eq!(rows(&pool).await, before);
            assert_eq!(before.len(), 5);
            assert_eq!(
                pragma(&pool, "SELECT name FROM sqlite_master WHERE type = 'index' AND tbl_name = 'tool_calls' AND name NOT LIKE 'sqlite_%' ORDER BY name").await,
                indexes
            );
            assert_eq!(indexes.len(), 6);
            let violations: Vec<(String,)> = sqlx::query_as("PRAGMA foreign_key_check")
                .fetch_all(&pool)
                .await
                .unwrap();
            assert!(violations.is_empty());
            assert_eq!(pragma(&pool, "PRAGMA integrity_check").await, ["ok"]);
            assert_eq!(
                pragma(
                    &pool,
                    "SELECT name FROM sqlite_master WHERE name = 'tool_calls_rebuilt'"
                )
                .await
                .len(),
                0
            );
            assert!(
                exec(
                    &pool,
                    &call(&[
                        ("id", "'bad'"),
                        ("position", "9"),
                        ("provider_call_id", "'p9'"),
                        ("worktree_id", "'nope'")
                    ])
                )
                .await
                .is_err()
            );
            exec(&pool, "DELETE FROM rooms WHERE id = 'r2'")
                .await
                .unwrap();
            assert_eq!(count(&pool, "tool_calls").await, 4);
            exec(&pool, "DELETE FROM task_worktrees WHERE id = 'k1'")
                .await
                .unwrap();
            assert_eq!(count(&pool, "tool_calls").await, 0);
        }

        #[tokio::test]
        async fn only_rejected_calls_may_have_null_tool_or_arguments() {
            let (pool, _dir) = seeded().await;
            let big = format!("'{{\"p\":\"{}\"}}'", "a".repeat(4090));
            let finished_failed = |code: &'static str| -> Vec<(&'static str, String)> {
                vec![
                    ("status", "'failed'".into()),
                    ("error_code", code.into()),
                    ("finished_at", LATER.into()),
                ]
            };
            let rejected =
                || -> Vec<(&'static str, String)> { finished_failed("'invalid_arguments'") };
            let with = |extra: Vec<(&'static str, String)>, base: Vec<(&'static str, String)>| {
                [base, extra].concat()
            };
            type Overrides = Vec<(&'static str, String)>;
            let cases: Vec<(&str, Overrides, bool)> = vec![
                (
                    "rejected, no tool and no arguments",
                    with(
                        vec![("tool", "NULL".into()), ("arguments", "NULL".into())],
                        rejected(),
                    ),
                    true,
                ),
                (
                    "rejected, tool kept",
                    with(vec![("arguments", "NULL".into())], rejected()),
                    true,
                ),
                (
                    "rejected, arguments kept",
                    with(
                        vec![("tool", "NULL".into()), ("arguments", "'{}'".into())],
                        rejected(),
                    ),
                    true,
                ),
                (
                    "valid failure with a tool code",
                    finished_failed("'path_outside'"),
                    true,
                ),
                (
                    "valid failure with invalid_arguments keeping its arguments",
                    rejected(),
                    true,
                ),
                (
                    "valid success",
                    vec![
                        ("status", "'succeeded'".into()),
                        ("result_bytes", "5".into()),
                        ("finished_at", LATER.into()),
                    ],
                    true,
                ),
                (
                    "NULL tool with another error code",
                    with(
                        vec![("tool", "NULL".into()), ("arguments", "NULL".into())],
                        finished_failed("'path_outside'"),
                    ),
                    false,
                ),
                (
                    "NULL arguments, cancelled",
                    vec![
                        ("arguments", "NULL".into()),
                        ("status", "'cancelled'".into()),
                        ("finished_at", LATER.into()),
                    ],
                    false,
                ),
                (
                    "NULL arguments, running",
                    vec![("arguments", "NULL".into())],
                    false,
                ),
                (
                    "NULL arguments, succeeded",
                    vec![
                        ("arguments", "NULL".into()),
                        ("status", "'succeeded'".into()),
                        ("result_bytes", "5".into()),
                        ("finished_at", LATER.into()),
                    ],
                    false,
                ),
                (
                    "NULL tool, failed with a NULL error code",
                    vec![
                        ("tool", "NULL".into()),
                        ("arguments", "NULL".into()),
                        ("status", "'failed'".into()),
                        ("error_code", "NULL".into()),
                        ("finished_at", LATER.into()),
                    ],
                    false,
                ),
                (
                    "invalid_arguments on a succeeded row",
                    vec![
                        ("tool", "NULL".into()),
                        ("arguments", "NULL".into()),
                        ("status", "'succeeded'".into()),
                        ("error_code", "'invalid_arguments'".into()),
                        ("result_bytes", "5".into()),
                        ("finished_at", LATER.into()),
                    ],
                    false,
                ),
                (
                    "rejected with a result size",
                    with(
                        vec![
                            ("tool", "NULL".into()),
                            ("arguments", "NULL".into()),
                            ("result_bytes", "5".into()),
                        ],
                        rejected(),
                    ),
                    false,
                ),
                (
                    "rejected truncated",
                    with(
                        vec![
                            ("tool", "NULL".into()),
                            ("arguments", "NULL".into()),
                            ("truncated", "1".into()),
                        ],
                        rejected(),
                    ),
                    false,
                ),
                (
                    "invalid tool name",
                    with(vec![("tool", "'Bad-Name'".into())], rejected()),
                    false,
                ),
                (
                    "oversized arguments",
                    with(vec![("arguments", big)], rejected()),
                    false,
                ),
                (
                    "array arguments",
                    with(vec![("arguments", "'[]'".into())], rejected()),
                    false,
                ),
                (
                    "arguments that are not JSON",
                    with(vec![("arguments", "'oops'".into())], rejected()),
                    false,
                ),
            ];
            assert_eq!(cases.len(), 18);
            for (index, (name, overrides, accepted)) in cases.iter().enumerate() {
                let id = format!("'n{index}'");
                let position = (index + 1).to_string();
                let provider = format!("'p{index}'");
                let mut all: Vec<(&str, &str)> = vec![
                    ("id", id.as_str()),
                    ("position", position.as_str()),
                    ("provider_call_id", provider.as_str()),
                ];
                all.extend(overrides.iter().map(|(key, value)| (*key, value.as_str())));
                assert_eq!(exec(&pool, &call(&all)).await.is_ok(), *accepted, "{name}");
            }
        }

        #[tokio::test]
        async fn a_failure_in_the_middle_of_the_migration_leaves_the_database_untouched() {
            for (label, broken) in [
                (
                    "after the drop",
                    MIGRATION.replacen("DROP TABLE tool_calls;", "DROP TABLE tool_calls;\nSELECT * FROM table_that_does_not_exist;", 1),
                ),
                (
                    "while copying",
                    MIGRATION.replacen(
                        "FROM tool_calls;",
                        "FROM tool_calls UNION ALL SELECT 'bad', 'run1', 'r1', 'w1', 't1', 'k1', 0, 1, 'x', 'list_dir', '{}', 'running', NULL, NULL, 0, 'x', NULL;",
                        1,
                    ),
                ),
            ] {
                let (pool, _dir) = pool_before_the_migration().await;
                seed(&pool).await;
                existing_rows(&pool).await;
                let (before_rows, before_schema) = (rows(&pool).await, schema(&pool).await);
                let mut migrations: Vec<Migration> = sqlx::migrate!("./migrations").migrations.iter().cloned().collect();
                let last = migrations.pop().unwrap();
                migrations.push(Migration::new(
                    last.version,
                    last.description.clone(),
                    MigrationType::Simple,
                    AssertSqlSafe(broken).into_sql_str(),
                    false,
                ));
                let attempt = Migrator {
                    migrations: Cow::Owned(migrations),
                    ..sqlx::migrate!("./migrations")
                }
                .run(&pool)
                .await;

                assert!(attempt.is_err(), "{label}");
                assert_eq!(rows(&pool).await, before_rows, "{label}");
                assert_eq!(schema(&pool).await, before_schema, "{label}");
                assert!(pragma(&pool, "SELECT name FROM sqlite_master WHERE name = 'tool_calls_rebuilt'").await.is_empty(), "{label}");
                let recorded: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations WHERE version = ?")
                    .bind(VERSION)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                assert_eq!(recorded, 0, "{label}");

                upgrade(&pool).await;
                assert_eq!(rows(&pool).await, before_rows, "{label}");
            }
        }

        #[tokio::test]
        async fn the_rollback_drops_only_rejected_rows_and_restores_the_original_schema() {
            let (pool, _dir) = pool_before_the_migration().await;
            seed(&pool).await;
            existing_rows(&pool).await;
            let (before_rows, before_schema) = (rows(&pool).await, schema(&pool).await);
            upgrade(&pool).await;
            sqlx::query("INSERT INTO tool_calls (id, run_id, room_id, workspace_id, task_id, worktree_id, iteration, position, provider_call_id, tool, arguments, status, error_code, started_at, finished_at) VALUES ('rej', 'run1', 'r1', 'w1', 't1', 'k1', 1, 50, 'call_rej', NULL, NULL, 'failed', 'invalid_arguments', ?, ?)")
                .bind("2026-11-19T10:00:00.000Z")
                .bind("2026-11-19T10:00:01.000Z")
                .execute(&pool)
                .await
                .unwrap();
            assert_eq!(count(&pool, "tool_calls").await, 6);

            let mut transaction = pool.begin().await.unwrap();
            sqlx::raw_sql(AssertSqlSafe(rollback_sql()))
                .execute(&mut *transaction)
                .await
                .unwrap();
            transaction.commit().await.unwrap();

            assert_eq!(rows(&pool).await, before_rows);
            assert_eq!(schema(&pool).await, before_schema);
            let violations: Vec<(String,)> = sqlx::query_as("PRAGMA foreign_key_check")
                .fetch_all(&pool)
                .await
                .unwrap();
            assert!(violations.is_empty());
            assert!(
                exec(
                    &pool,
                    &call(&[
                        ("id", "'again'"),
                        ("position", "60"),
                        ("provider_call_id", "'p60'"),
                        ("tool", "NULL"),
                        ("arguments", "NULL"),
                        ("status", "'failed'"),
                        ("error_code", "'invalid_arguments'"),
                        ("finished_at", LATER)
                    ])
                )
                .await
                .is_err()
            );
            upgrade_again(&pool).await;
            assert_eq!(rows(&pool).await, before_rows);
        }

        async fn upgrade_again(pool: &SqlitePool) {
            sqlx::query("DELETE FROM _sqlx_migrations WHERE version = ?")
                .bind(VERSION)
                .execute(pool)
                .await
                .unwrap();
            upgrade(pool).await;
        }
    }
}
