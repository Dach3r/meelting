#[cfg(test)]
mod tests {
    use crate::repositories::database::open_for_test;
    use sqlx::SqlitePool;
    use sqlx::migrate::Migrator;
    use std::borrow::Cow;

    const SEED: [&str; 6] = [
        "INSERT INTO workspaces VALUES ('w1', 'A', 'x', 'x'), ('w2', 'B', 'x', 'x')",
        "INSERT INTO teams VALUES ('t1', 'w1', 'General', 'x', 'x'), ('t2', 'w2', 'Other', 'x', 'x')",
        "INSERT INTO agents VALUES ('a1', 'w1', 't1', NULL, 'Ana', 'QA', '', '', 'ollama', 'qwen', 'x', 'x'), ('a2', 'w1', 't1', NULL, 'Bob', 'Dev', '', '', 'ollama', 'qwen', 'x', 'x'), ('a3', 'w2', 't2', NULL, 'Eva', 'QA', '', '', 'ollama', 'qwen', 'x', 'x'), ('a4', 'w1', 't1', NULL, 'Idle', 'QA', '', '', 'ollama', 'qwen', 'x', 'x')",
        "INSERT INTO rooms VALUES ('r1', 'w1', 'Sala', 'x', 'x'), ('r2', 'w1', 'Otra', 'x', 'x'), ('r3', 'w2', 'Ajena', 'x', 'x')",
        "INSERT INTO room_agents VALUES ('r1', 'a1', 'w1', 'x'), ('r1', 'a2', 'w1', 'x'), ('r2', 'a1', 'w1', 'x'), ('r3', 'a3', 'w2', 'x'), ('r1', 'a4', 'w1', 'x')",
        "INSERT INTO messages VALUES ('u1', 'r1', NULL, 'hola', 'x'), ('u2', 'r1', NULL, 'otra', 'x'), ('u3', 'r2', NULL, 'hola', 'x'), ('u4', 'r3', NULL, 'hola', 'x'), ('g1', 'r1', 'a1', 'hi', 'x'), ('g2', 'r1', 'a2', 'hi', 'x'), ('g3', 'r2', 'a1', 'hi', 'x'), ('g4', 'r3', 'a3', 'hi', 'x')",
    ];

    const COLUMNS: [&str; 16] = [
        "id",
        "room_id",
        "agent_id",
        "request_message_id",
        "response_message_id",
        "client_request_id",
        "status",
        "error_kind",
        "attempts",
        "provider",
        "model",
        "input_tokens",
        "output_tokens",
        "created_at",
        "started_at",
        "finished_at",
    ];

    const T0: &str = "'2026-11-19T10:00:00.000Z'";
    const T1: &str = "'2026-11-19T10:00:01.000Z'";
    const T2: &str = "'2026-11-19T10:00:02.000Z'";
    const BEFORE: &str = "'2026-11-19T09:59:59.000Z'";

    async fn seeded() -> (SqlitePool, tempfile::TempDir) {
        let (pool, dir) = open_for_test().await;
        for sql in SEED {
            sqlx::query(sql).execute(&pool).await.unwrap();
        }
        (pool, dir)
    }

    fn run(overrides: &[(&str, &str)]) -> String {
        let values: Vec<String> = COLUMNS
            .iter()
            .map(|column| {
                overrides
                    .iter()
                    .rfind(|(name, _)| name == column)
                    .map(|(_, value)| value.to_string())
                    .unwrap_or_else(|| default_value(column).to_string())
            })
            .collect();
        format!(
            "INSERT INTO agent_runs ({}) VALUES ({})",
            COLUMNS.join(", "),
            values.join(", ")
        )
    }

    fn default_value(column: &str) -> &'static str {
        match column {
            "id" => "'run1'",
            "room_id" => "'r1'",
            "agent_id" => "'a1'",
            "request_message_id" => "'u1'",
            "client_request_id" => "'0196f3a0-0000-7000-8000-000000000001'",
            "status" => "'queued'",
            "attempts" => "0",
            "provider" => "'ollama'",
            "model" => "'qwen'",
            "created_at" => T0,
            _ => "NULL",
        }
    }

    async fn exec(pool: &SqlitePool, sql: &str) -> Result<u64, sqlx::Error> {
        sqlx::query(sqlx::AssertSqlSafe(sql.to_string()))
            .execute(pool)
            .await
            .map(|result| result.rows_affected())
    }

    async fn count(pool: &SqlitePool, sql: &'static str) -> i64 {
        sqlx::query_scalar(sql).fetch_one(pool).await.unwrap()
    }

    async fn foreign_key_violations(pool: &SqlitePool) -> usize {
        sqlx::query("SELECT 1 FROM pragma_foreign_key_check")
            .fetch_all(pool)
            .await
            .unwrap()
            .len()
    }

    fn running(attempts: &'static str) -> Vec<(&'static str, &'static str)> {
        vec![
            ("status", "'running'"),
            ("attempts", attempts),
            ("started_at", T1),
        ]
    }

    #[tokio::test]
    async fn single_row_matrix() {
        let (pool, _dir) = seeded().await;
        let succeeded = [
            ("status", "'succeeded'"),
            ("attempts", "1"),
            ("started_at", T1),
            ("finished_at", T2),
            ("response_message_id", "'g1'"),
        ];
        let cases: Vec<(&str, Vec<(&str, &str)>, bool)> = vec![
            ("queued without attempts", vec![], true),
            ("running on its first attempt", running("1"), true),
            (
                "queued waiting for a retry",
                vec![("attempts", "2"), ("started_at", T1)],
                true,
            ),
            (
                "succeeded with its response and tokens",
                [
                    succeeded.to_vec(),
                    vec![("input_tokens", "10"), ("output_tokens", "0")],
                ]
                .concat(),
                true,
            ),
            (
                "cancelled while queued",
                vec![
                    ("status", "'failed'"),
                    ("error_kind", "'cancelled'"),
                    ("finished_at", T2),
                ],
                true,
            ),
            (
                "expired after two attempts",
                vec![
                    ("status", "'failed'"),
                    ("error_kind", "'expired'"),
                    ("attempts", "2"),
                    ("started_at", T1),
                    ("finished_at", T2),
                ],
                true,
            ),
            (
                "failed after the last attempt",
                vec![
                    ("status", "'failed'"),
                    ("error_kind", "'rate_limited'"),
                    ("attempts", "3"),
                    ("started_at", T1),
                    ("finished_at", T2),
                ],
                true,
            ),
            (
                "context too large is a known error",
                vec![
                    ("status", "'failed'"),
                    ("error_kind", "'context_too_large'"),
                    ("attempts", "1"),
                    ("started_at", T1),
                    ("finished_at", T1),
                ],
                true,
            ),
            (
                "an agent message as request is only stopped by the service",
                vec![("request_message_id", "'g2'")],
                true,
            ),
            (
                "request from another room",
                vec![("request_message_id", "'u3'")],
                false,
            ),
            (
                "request that does not exist",
                vec![("request_message_id", "'missing'")],
                false,
            ),
            (
                "request from another workspace",
                vec![("request_message_id", "'u4'")],
                false,
            ),
            (
                "response from another room",
                [succeeded.to_vec(), vec![("response_message_id", "'g3'")]].concat(),
                false,
            ),
            (
                "response from another agent",
                [succeeded.to_vec(), vec![("response_message_id", "'g2'")]].concat(),
                false,
            ),
            (
                "response from the user",
                [succeeded.to_vec(), vec![("response_message_id", "'u2'")]].concat(),
                false,
            ),
            (
                "response from another workspace",
                [succeeded.to_vec(), vec![("response_message_id", "'g4'")]].concat(),
                false,
            ),
            (
                "response equal to the request",
                [
                    succeeded.to_vec(),
                    vec![
                        ("request_message_id", "'g1'"),
                        ("response_message_id", "'g1'"),
                    ],
                ]
                .concat(),
                false,
            ),
            (
                "agent that is not in the room",
                vec![
                    ("room_id", "'r2'"),
                    ("agent_id", "'a2'"),
                    ("request_message_id", "'u3'"),
                ],
                false,
            ),
            (
                "agent of another workspace",
                vec![("agent_id", "'a3'")],
                false,
            ),
            (
                "room that does not exist",
                vec![("room_id", "'missing'")],
                false,
            ),
            (
                "uppercase client request id",
                vec![(
                    "client_request_id",
                    "'0196F3A0-0000-7000-8000-000000000001'",
                )],
                false,
            ),
            (
                "short client request id",
                vec![("client_request_id", "'0196f3a0-0000-7000-8000-00000000001'")],
                false,
            ),
            ("unknown status", vec![("status", "'cancelled'")], false),
            (
                "unknown error kind",
                vec![
                    ("status", "'failed'"),
                    ("error_kind", "'exploded'"),
                    ("finished_at", T2),
                ],
                false,
            ),
            (
                "failed without error kind",
                vec![("status", "'failed'"), ("finished_at", T2)],
                false,
            ),
            (
                "queued with error kind",
                vec![("error_kind", "'timeout'")],
                false,
            ),
            (
                "succeeded without response",
                [succeeded.to_vec(), vec![("response_message_id", "NULL")]].concat(),
                false,
            ),
            (
                "failed with response",
                [
                    succeeded.to_vec(),
                    vec![("status", "'failed'"), ("error_kind", "'timeout'")],
                ]
                .concat(),
                false,
            ),
            (
                "running with finished_at",
                [running("1"), vec![("finished_at", T2)]].concat(),
                false,
            ),
            (
                "succeeded without finished_at",
                [succeeded.to_vec(), vec![("finished_at", "NULL")]].concat(),
                false,
            ),
            (
                "running without attempts",
                vec![("status", "'running'")],
                false,
            ),
            (
                "succeeded without attempts",
                [
                    succeeded.to_vec(),
                    vec![("attempts", "0"), ("started_at", "NULL")],
                ]
                .concat(),
                false,
            ),
            (
                "attempts without started_at",
                vec![("attempts", "1")],
                false,
            ),
            (
                "started_at without attempts",
                vec![("started_at", T1)],
                false,
            ),
            (
                "queued with all attempts used",
                vec![("attempts", "3"), ("started_at", T1)],
                false,
            ),
            ("four attempts", running("4"), false),
            ("negative attempts", vec![("attempts", "-1")], false),
            (
                "started before created",
                [running("1"), vec![("started_at", BEFORE)]].concat(),
                false,
            ),
            (
                "finished before created",
                vec![
                    ("status", "'failed'"),
                    ("error_kind", "'cancelled'"),
                    ("finished_at", BEFORE),
                ],
                false,
            ),
            (
                "finished before started",
                [
                    succeeded.to_vec(),
                    vec![("started_at", T2), ("finished_at", T1)],
                ]
                .concat(),
                false,
            ),
            ("blank provider", vec![("provider", "'  '")], false),
            ("blank model", vec![("model", "''")], false),
            ("negative input tokens", vec![("input_tokens", "-1")], false),
            (
                "negative output tokens",
                vec![("output_tokens", "-1")],
                false,
            ),
            ("text in attempts", vec![("attempts", "'one'")], false),
        ];

        for (name, overrides, accepted) in cases {
            let mut transaction = pool.begin().await.unwrap();
            let result = sqlx::query(sqlx::AssertSqlSafe(run(&overrides)))
                .execute(&mut *transaction)
                .await;
            transaction.rollback().await.unwrap();
            assert_eq!(result.is_ok(), accepted, "{name}: {result:?}");
        }
    }

    #[tokio::test]
    async fn client_request_ids_are_unique_per_room() {
        let (pool, _dir) = seeded().await;
        exec(&pool, &run(&[])).await.unwrap();

        let same_room = run(&[
            ("id", "'run2'"),
            ("agent_id", "'a2'"),
            ("request_message_id", "'u2'"),
        ]);
        assert!(exec(&pool, &same_room).await.is_err());

        let other_room = run(&[
            ("id", "'run3'"),
            ("room_id", "'r2'"),
            ("request_message_id", "'u3'"),
        ]);
        exec(&pool, &other_room).await.unwrap();
    }

    #[tokio::test]
    async fn an_agent_has_one_active_run_per_room() {
        let (pool, _dir) = seeded().await;
        exec(&pool, &run(&[])).await.unwrap();

        let second_active = run(&[
            ("id", "'run2'"),
            ("request_message_id", "'u2'"),
            (
                "client_request_id",
                "'0196f3a0-0000-7000-8000-000000000002'",
            ),
        ]);
        assert!(exec(&pool, &second_active).await.is_err());

        exec(
            &pool,
            &format!(
                "UPDATE agent_runs SET status = 'failed', error_kind = 'cancelled', finished_at = {T2}"
            ),
        )
        .await
        .unwrap();
        exec(&pool, &second_active).await.unwrap();

        let other_room = run(&[
            ("id", "'run3'"),
            ("room_id", "'r2'"),
            ("request_message_id", "'u3'"),
            (
                "client_request_id",
                "'0196f3a0-0000-7000-8000-000000000003'",
            ),
        ]);
        exec(&pool, &other_room).await.unwrap();
    }

    #[tokio::test]
    async fn requests_and_responses_belong_to_a_single_run() {
        let (pool, _dir) = seeded().await;
        let finished = |id: &'static str, client: &'static str, request: &'static str| {
            run(&[
                ("id", id),
                ("client_request_id", client),
                ("request_message_id", request),
                ("status", "'succeeded'"),
                ("attempts", "1"),
                ("started_at", T1),
                ("finished_at", T2),
                ("response_message_id", "'g1'"),
            ])
        };
        exec(
            &pool,
            &finished("'run1'", "'0196f3a0-0000-7000-8000-000000000001'", "'u1'"),
        )
        .await
        .unwrap();

        let reused_response = finished("'run2'", "'0196f3a0-0000-7000-8000-000000000002'", "'u2'");
        assert!(exec(&pool, &reused_response).await.is_err());

        let reused_request = run(&[
            ("id", "'run3'"),
            (
                "client_request_id",
                "'0196f3a0-0000-7000-8000-000000000003'",
            ),
            ("agent_id", "'a2'"),
        ]);
        assert!(exec(&pool, &reused_request).await.is_err());
    }

    #[tokio::test]
    async fn a_guarded_transition_never_leaves_a_final_state() {
        let (pool, _dir) = seeded().await;
        exec(
            &pool,
            &run(&[
                ("status", "'failed'"),
                ("error_kind", "'cancelled'"),
                ("finished_at", T2),
            ]),
        )
        .await
        .unwrap();

        let changed = exec(
            &pool,
            &format!(
                "UPDATE agent_runs SET status = 'running', attempts = 1, started_at = {T1}, error_kind = NULL, finished_at = NULL WHERE id = 'run1' AND status = 'queued'"
            ),
        )
        .await
        .unwrap();

        assert_eq!(changed, 0);
        let status: String = sqlx::query_scalar("SELECT status FROM agent_runs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(status, "failed");
    }

    #[tokio::test]
    async fn deleting_a_room_removes_its_runs_even_while_active() {
        let (pool, _dir) = seeded().await;
        exec(&pool, &run(&running("1"))).await.unwrap();
        exec(
            &pool,
            &run(&[
                ("id", "'run2'"),
                ("agent_id", "'a2'"),
                ("request_message_id", "'u2'"),
                (
                    "client_request_id",
                    "'0196f3a0-0000-7000-8000-000000000002'",
                ),
                ("status", "'succeeded'"),
                ("attempts", "1"),
                ("started_at", T1),
                ("finished_at", T2),
                ("response_message_id", "'g2'"),
            ]),
        )
        .await
        .unwrap();
        exec(
            &pool,
            &run(&[
                ("id", "'run3'"),
                ("room_id", "'r2'"),
                ("request_message_id", "'u3'"),
            ]),
        )
        .await
        .unwrap();

        exec(&pool, "DELETE FROM rooms WHERE id = 'r1'")
            .await
            .unwrap();

        assert_eq!(
            count(
                &pool,
                "SELECT COUNT(*) FROM agent_runs WHERE room_id = 'r1'"
            )
            .await,
            0
        );
        assert_eq!(
            count(&pool, "SELECT COUNT(*) FROM messages WHERE room_id = 'r1'").await,
            0
        );
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM agent_runs").await, 1);
        assert_eq!(foreign_key_violations(&pool).await, 0);
    }

    #[tokio::test]
    async fn deleting_a_workspace_removes_its_runs() {
        let (pool, _dir) = seeded().await;
        exec(&pool, &run(&[])).await.unwrap();

        exec(&pool, "DELETE FROM workspaces WHERE id = 'w1'")
            .await
            .unwrap();

        assert_eq!(count(&pool, "SELECT COUNT(*) FROM agent_runs").await, 0);
        assert_eq!(foreign_key_violations(&pool).await, 0);
    }

    #[tokio::test]
    async fn an_agent_with_runs_and_no_messages_cannot_be_deleted_nor_leave_the_room() {
        let (pool, _dir) = seeded().await;
        exec(&pool, &run(&[("agent_id", "'a4'")])).await.unwrap();

        assert!(
            exec(&pool, "DELETE FROM agents WHERE id = 'a4'")
                .await
                .is_err()
        );
        assert!(
            exec(&pool, "DELETE FROM room_agents WHERE agent_id = 'a4'")
                .await
                .is_err()
        );
        assert_eq!(
            count(&pool, "SELECT COUNT(*) FROM agents WHERE id = 'a4'").await,
            1
        );
    }

    #[tokio::test]
    async fn an_agent_without_messages_nor_runs_is_still_deleted() {
        let (pool, _dir) = seeded().await;
        exec(&pool, &run(&[])).await.unwrap();

        exec(&pool, "DELETE FROM agents WHERE id = 'a4'")
            .await
            .unwrap();

        assert_eq!(
            count(
                &pool,
                "SELECT COUNT(*) FROM room_agents WHERE agent_id = 'a4'"
            )
            .await,
            0
        );
    }

    #[tokio::test]
    async fn messages_referenced_by_a_run_cannot_be_deleted_one_by_one() {
        let (pool, _dir) = seeded().await;
        exec(
            &pool,
            &run(&[
                ("status", "'succeeded'"),
                ("attempts", "1"),
                ("started_at", T1),
                ("finished_at", T2),
                ("response_message_id", "'g1'"),
            ]),
        )
        .await
        .unwrap();

        assert!(
            exec(&pool, "DELETE FROM messages WHERE id = 'u1'")
                .await
                .is_err()
        );
        assert!(
            exec(&pool, "DELETE FROM messages WHERE id = 'g1'")
                .await
                .is_err()
        );
        exec(&pool, "DELETE FROM messages WHERE id = 'u2'")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn expiring_active_runs_reads_only_the_partial_index() {
        let (pool, _dir) = seeded().await;

        let rows: Vec<(i64, i64, i64, String)> = sqlx::query_as(
            "EXPLAIN QUERY PLAN SELECT id FROM agent_runs WHERE status IN ('queued', 'running')",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        let plan: Vec<String> = rows.into_iter().map(|(_, _, _, detail)| detail).collect();

        assert!(
            plan.iter()
                .any(|detail| detail.contains("agent_runs_active_room_id_agent_id")),
            "{plan:?}"
        );
    }

    #[tokio::test]
    async fn the_new_migrations_apply_over_a_database_with_data() {
        let dir = tempfile::tempdir().unwrap();
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(dir.path().join("test.db"))
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePool::connect_with(options).await.unwrap();
        let full: Migrator = sqlx::migrate!("./migrations");
        let mut before = sqlx::migrate!("./migrations");
        before.migrations = Cow::Owned(
            full.migrations
                .iter()
                .filter(|migration| migration.version < 20261119000001)
                .cloned()
                .collect(),
        );
        before.run(&pool).await.unwrap();
        for sql in SEED {
            sqlx::query(sql).execute(&pool).await.unwrap();
        }

        full.run(&pool).await.unwrap();

        exec(&pool, &run(&[])).await.unwrap();
        assert_eq!(foreign_key_violations(&pool).await, 0);
        let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(integrity, "ok");
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM messages").await, 8);
    }
}
