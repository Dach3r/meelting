#[cfg(test)]
mod tests {
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
            exec(&pool, sql).await.unwrap();
        }
        exec(&pool, &worktree("k1", "t1", "w1")).await.unwrap();
        exec(&pool, &worktree("k2", "t2", "w1")).await.unwrap();
        (pool, dir)
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
}
