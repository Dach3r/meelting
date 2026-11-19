use crate::domain::task::{Task, TaskDraft, TaskPriority, TaskStatus};
use sqlx::SqliteExecutor;

const SELECT: &str = "SELECT tasks.id, tasks.title, tasks.description, tasks.status, tasks.priority, tasks.repository_id, tasks.base_ref, tasks.origin_room_id, rooms.name AS origin_room_name, tasks.origin_message_id, tasks.completed_at, tasks.created_at, tasks.updated_at \
     FROM tasks LEFT JOIN rooms ON rooms.id = tasks.origin_room_id";
const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    title: String,
    description: String,
    status: String,
    priority: String,
    repository_id: Option<String>,
    base_ref: Option<String>,
    origin_room_id: Option<String>,
    origin_room_name: Option<String>,
    origin_message_id: Option<String>,
    completed_at: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TryFrom<Row> for Task {
    type Error = sqlx::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        let invalid = |column: &str| sqlx::Error::ColumnDecode {
            index: column.into(),
            source: "unknown value".into(),
        };
        Ok(Task {
            status: TaskStatus::parse(&row.status).ok_or_else(|| invalid("status"))?,
            priority: TaskPriority::parse(&row.priority).ok_or_else(|| invalid("priority"))?,
            id: row.id,
            title: row.title,
            description: row.description,
            repository_id: row.repository_id,
            base_ref: row.base_ref,
            origin_room_id: row.origin_room_id,
            origin_room_name: row.origin_room_name,
            origin_message_id: row.origin_message_id,
            agent_ids: Vec::new(),
            worktree: None,
            completed_at: row.completed_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub async fn list<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
) -> Result<Vec<Task>, sqlx::Error> {
    let rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT} WHERE tasks.workspace_id = ? ORDER BY tasks.updated_at DESC, tasks.id DESC"
    )))
    .bind(workspace_id)
    .fetch_all(executor)
    .await?;
    rows.into_iter().map(Task::try_from).collect()
}

pub async fn get<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
    id: &str,
) -> Result<Option<Task>, sqlx::Error> {
    let row: Option<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT} WHERE tasks.id = ? AND tasks.workspace_id = ?"
    )))
    .bind(id)
    .bind(workspace_id)
    .fetch_optional(executor)
    .await?;
    row.map(Task::try_from).transpose()
}

pub async fn insert<'e>(
    executor: impl SqliteExecutor<'e>,
    id: &str,
    workspace_id: &str,
    draft: &TaskDraft,
    origin_room_id: Option<&str>,
    origin_message_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO tasks (id, workspace_id, repository_id, origin_room_id, origin_message_id, title, description, status, priority, base_ref, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?, {NOW}, {NOW})"
    )))
    .bind(id)
    .bind(workspace_id)
    .bind(&draft.repository_id)
    .bind(origin_room_id)
    .bind(origin_message_id)
    .bind(&draft.title)
    .bind(&draft.description)
    .bind(draft.priority.as_str())
    .bind(&draft.base_ref)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn update<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
    id: &str,
    draft: &TaskDraft,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE tasks SET title = ?, description = ?, priority = ?, repository_id = ?, base_ref = ?, updated_at = {NOW} \
         WHERE id = ? AND workspace_id = ?"
    )))
    .bind(&draft.title)
    .bind(&draft.description)
    .bind(draft.priority.as_str())
    .bind(&draft.repository_id)
    .bind(&draft.base_ref)
    .bind(id)
    .bind(workspace_id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn change_status<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
    id: &str,
    from: TaskStatus,
    to: TaskStatus,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE tasks SET status = ?1, completed_at = CASE WHEN ?1 = 'completed' THEN {NOW} END, updated_at = {NOW} \
         WHERE id = ?2 AND workspace_id = ?3 AND status = ?4"
    )))
    .bind(to.as_str())
    .bind(id)
    .bind(workspace_id)
    .bind(from.as_str())
    .execute(executor)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
    id: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM tasks WHERE id = ? AND workspace_id = ?")
        .bind(id)
        .bind(workspace_id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use crate::repositories::database::open_for_test;
    use sqlx::SqlitePool;

    async fn exec(pool: &SqlitePool, sql: &str) -> Result<(), sqlx::Error> {
        sqlx::query(sqlx::AssertSqlSafe(sql.to_string()))
            .execute(pool)
            .await
            .map(|_| ())
    }

    async fn count(pool: &SqlitePool, table: &str) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn seeded() -> (SqlitePool, tempfile::TempDir) {
        let (pool, dir) = open_for_test().await;
        for sql in [
            "INSERT INTO workspaces VALUES ('w1', 'A', 'x', 'x'), ('w2', 'B', 'x', 'x')",
            "INSERT INTO teams VALUES ('t1', 'w1', 'General', 'x', 'x'), ('t2', 'w2', 'Other', 'x', 'x')",
            "INSERT INTO agents (id, workspace_id, team_id, name, role, description, instructions, provider, model, created_at, updated_at) \
             VALUES ('a1', 'w1', 't1', 'Ada', 'Dev', '', '', 'local', 'qwen', 'x', 'x'), \
                    ('a2', 'w2', 't2', 'Bob', 'Dev', '', '', 'local', 'qwen', 'x', 'x')",
            "INSERT INTO rooms (id, workspace_id, name, created_at, updated_at) VALUES ('r1', 'w1', 'Daily', 'x', 'x'), ('r2', 'w1', 'Other', 'x', 'x'), ('r3', 'w2', 'Far', 'x', 'x')",
            "INSERT INTO messages (id, room_id, agent_id, body, created_at) VALUES ('m1', 'r1', NULL, 'Hi', 'x'), ('m2', 'r2', NULL, 'Yo', 'x')",
            "INSERT INTO repositories VALUES ('p1', 'w1', 'app', '/app', '/app/.git', 'x', 'x'), ('p2', 'w2', 'app', '/app', '/app/.git', 'x', 'x')",
        ] {
            exec(&pool, sql).await.unwrap();
        }
        (pool, dir)
    }

    fn task(id: &str, workspace: &str, extra: &str) -> String {
        let (columns, values) = extra.split_once('|').unwrap_or(("", ""));
        format!(
            "INSERT INTO tasks (id, workspace_id, title, description, status, priority, created_at, updated_at{columns}) \
             VALUES ('{id}', '{workspace}', 'Fix', '', 'pending', 'low', 'x', 'x'{values})"
        )
    }

    #[tokio::test]
    async fn tasks_enforce_text_status_priority_and_completion_rules() {
        let (pool, _dir) = seeded().await;
        let long_title = format!("'{}'", "a".repeat(121));
        let long_description = format!("'{}'", "a".repeat(10001));
        for (index, sql) in [
            task("x0", "w1", "").replace("'Fix'", "'  '"),
            task("x1", "w1", "").replace("'Fix'", &long_title),
            task("x2", "w1", "").replace("''", &long_description),
            task("x3", "w1", "").replace("'pending'", "'done'"),
            task("x4", "w1", "").replace("'low'", "'urgent'"),
            task("x5", "w1", "").replace("'pending'", "'completed'"),
            task("x6", "w1", ", completed_at|, 'x'"),
            task("x7", "w1", ", base_ref|, 'main'"),
            task("x8", "w1", ", base_ref, repository_id|, '', 'p1'"),
            task("x9", "w1", ", origin_message_id|, 'm1'"),
        ]
        .iter()
        .enumerate()
        {
            assert!(exec(&pool, sql).await.is_err(), "case {index} was accepted");
        }
        let completed =
            task("ok", "w1", ", completed_at|, 'x'").replace("'pending'", "'completed'");
        exec(&pool, &completed).await.unwrap();
        assert_eq!(count(&pool, "tasks").await, 1);
    }

    #[tokio::test]
    async fn references_must_belong_to_the_task_workspace() {
        let (pool, _dir) = seeded().await;

        for (index, sql) in [
            task("x1", "w1", ", repository_id|, 'p2'"),
            task("x2", "w1", ", origin_room_id|, 'r3'"),
            task(
                "x3",
                "w1",
                ", origin_room_id, origin_message_id|, 'r1', 'm2'",
            ),
            task("x4", "w1", ", repository_id|, 'missing'"),
        ]
        .iter()
        .enumerate()
        {
            assert!(exec(&pool, sql).await.is_err(), "case {index} was accepted");
        }
        exec(&pool, &task("t1", "w1", ", repository_id, base_ref, origin_room_id, origin_message_id|, 'p1', 'main', 'r1', 'm1'"))
            .await
            .unwrap();
        assert!(
            exec(
                &pool,
                "UPDATE tasks SET repository_id = 'p2' WHERE id = 't1'"
            )
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn task_agents_must_share_the_workspace_and_be_unique() {
        let (pool, _dir) = seeded().await;
        exec(&pool, &task("t1", "w1", "")).await.unwrap();
        let assign = |agent: &str, workspace: &str| {
            format!("INSERT INTO task_agents VALUES ('t1', '{agent}', '{workspace}', 'x')")
        };

        assert!(exec(&pool, &assign("a2", "w1")).await.is_err());
        assert!(exec(&pool, &assign("a2", "w2")).await.is_err());
        exec(&pool, &assign("a1", "w1")).await.unwrap();
        assert!(exec(&pool, &assign("a1", "w1")).await.is_err());
    }

    #[tokio::test]
    async fn deleting_a_task_or_an_agent_removes_the_assignments() {
        let (pool, _dir) = seeded().await;
        exec(&pool, &task("t1", "w1", "")).await.unwrap();
        exec(&pool, &task("t2", "w1", "")).await.unwrap();
        exec(
            &pool,
            "INSERT INTO task_agents VALUES ('t1', 'a1', 'w1', 'x'), ('t2', 'a1', 'w1', 'x')",
        )
        .await
        .unwrap();

        exec(&pool, "DELETE FROM tasks WHERE id = 't1'")
            .await
            .unwrap();
        assert_eq!(count(&pool, "task_agents").await, 1);

        exec(&pool, "DELETE FROM agents WHERE id = 'a1'")
            .await
            .unwrap();
        assert_eq!(count(&pool, "task_agents").await, 0);
        assert_eq!(count(&pool, "tasks").await, 1);
    }

    #[tokio::test]
    async fn a_referenced_repository_cannot_be_deleted() {
        let (pool, _dir) = seeded().await;
        exec(&pool, &task("t1", "w1", ", repository_id|, 'p1'"))
            .await
            .unwrap();

        let error = exec(&pool, "DELETE FROM repositories WHERE id = 'p1'")
            .await
            .unwrap_err();

        assert!(
            error
                .as_database_error()
                .unwrap()
                .is_foreign_key_violation()
        );
        exec(&pool, "DELETE FROM tasks WHERE id = 't1'")
            .await
            .unwrap();
        exec(&pool, "DELETE FROM repositories WHERE id = 'p1'")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn deleting_a_workspace_removes_its_tasks_with_their_origin_and_repository() {
        let (pool, _dir) = seeded().await;
        exec(
            &pool,
            &task(
                "t1",
                "w1",
                ", repository_id, origin_room_id, origin_message_id|, 'p1', 'r1', 'm1'",
            ),
        )
        .await
        .unwrap();
        exec(
            &pool,
            "INSERT INTO task_agents VALUES ('t1', 'a1', 'w1', 'x')",
        )
        .await
        .unwrap();
        exec(&pool, &task("t2", "w2", ", repository_id|, 'p2'"))
            .await
            .unwrap();

        exec(&pool, "DELETE FROM workspaces WHERE id = 'w1'")
            .await
            .unwrap();

        assert_eq!(count(&pool, "tasks").await, 1);
        assert_eq!(count(&pool, "task_agents").await, 0);
        assert_eq!(count(&pool, "repositories").await, 1);
        assert_eq!(count(&pool, "rooms").await, 1);
    }

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    fn worktree(id: &str, task: &str, repository: &str, workspace: &str, status: &str) -> String {
        let removed_at = if status == "removed" { "'y'" } else { "NULL" };
        let error_code = if status == "failed" {
            "'checkout_failed'"
        } else {
            "NULL"
        };
        format!(
            "INSERT INTO task_worktrees (id, workspace_id, task_id, repository_id, path, branch_name, base_ref, base_commit_sha, status, error_code, removed_at, created_at, updated_at) \
             VALUES ('{id}', '{workspace}', '{task}', '{repository}', '/wt/{id}', 'meelting/fix-{id}', 'main', '{SHA}', '{status}', {error_code}, {removed_at}, 'x', 'x')"
        )
    }

    async fn with_worktree_task() -> (SqlitePool, tempfile::TempDir) {
        let (pool, dir) = seeded().await;
        exec(
            &pool,
            "INSERT INTO repositories VALUES ('p3', 'w1', 'other', '/o', '/o/.git', 'x', 'x')",
        )
        .await
        .unwrap();
        exec(&pool, &task("t1", "w1", ", repository_id|, 'p1'"))
            .await
            .unwrap();
        exec(&pool, &task("t2", "w1", "")).await.unwrap();
        (pool, dir)
    }

    #[tokio::test]
    async fn worktrees_use_the_repository_and_workspace_of_their_task() {
        let (pool, _dir) = with_worktree_task().await;

        for (index, sql) in [
            worktree("k1", "t1", "p3", "w1", "removed"),
            worktree("k2", "t1", "p1", "w2", "removed"),
            worktree("k3", "t2", "p1", "w1", "removed"),
            worktree("k4", "missing", "p1", "w1", "removed"),
        ]
        .iter()
        .enumerate()
        {
            assert!(exec(&pool, sql).await.is_err(), "case {index} was accepted");
        }
        exec(&pool, &worktree("ok", "t1", "p1", "w1", "ready"))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_task_has_at_most_one_active_worktree() {
        let (pool, _dir) = with_worktree_task().await;
        exec(&pool, &worktree("k1", "t1", "p1", "w1", "removed"))
            .await
            .unwrap();
        exec(&pool, &worktree("k2", "t1", "p1", "w1", "failed"))
            .await
            .unwrap();

        for (index, status) in ["preparing", "ready", "failed", "missing", "removing"]
            .iter()
            .enumerate()
        {
            let sql = worktree(&format!("x{index}"), "t1", "p1", "w1", status);
            assert!(exec(&pool, &sql).await.is_err(), "{status} was accepted");
        }
        exec(&pool, &worktree("k3", "t1", "p1", "w1", "removed"))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn worktree_rows_enforce_names_sha_status_and_dates() {
        let (pool, _dir) = with_worktree_task().await;
        let base = worktree("k1", "t1", "p1", "w1", "ready");

        for (index, sql) in [
            base.replace("'meelting/fix-k1'", "'feature/k1'"),
            base.replace("'meelting/fix-k1'", "'meelting/Fix-k1'"),
            base.replace("'meelting/fix-k1'", "'meelting/fix k1'"),
            base.replace(SHA, &SHA[..39]),
            base.replace(SHA, &SHA.to_uppercase()),
            base.replace("'ready'", "'done'"),
            base.replace("'ready', NULL", "'ready', 'oops'"),
            base.replace("'ready', NULL, NULL", "'ready', NULL, 'y'"),
            worktree("k1", "t1", "p1", "w1", "failed").replace("'checkout_failed'", "NULL"),
            worktree("k1", "t1", "p1", "w1", "failed").replace("'checkout_failed'", "'Bad Code'"),
            worktree("k1", "t1", "p1", "w1", "removed").replace("'y'", "NULL"),
            base.replace("'x', 'x')", "'x', 'w')"),
            base.replace("''", "'x'").replace("'/wt/k1'", "''"),
        ]
        .iter()
        .enumerate()
        {
            assert!(exec(&pool, sql).await.is_err(), "case {index} was accepted");
        }
        exec(&pool, &base).await.unwrap();
        assert!(
            exec(
                &pool,
                &worktree("k2", "t2", "p1", "w1", "removed").replace("'/wt/k2'", "'/wt/k1'")
            )
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn worktree_history_fixes_the_task_repository_and_blocks_deletions() {
        let (pool, _dir) = with_worktree_task().await;
        exec(&pool, &task("t3", "w1", ", repository_id|, 'p1'"))
            .await
            .unwrap();
        exec(
            &pool,
            "UPDATE tasks SET repository_id = 'p3' WHERE id = 't3'",
        )
        .await
        .unwrap();
        exec(&pool, &worktree("k1", "t1", "p1", "w1", "removed"))
            .await
            .unwrap();

        for sql in [
            "UPDATE tasks SET repository_id = 'p3' WHERE id = 't1'",
            "UPDATE tasks SET repository_id = NULL WHERE id = 't1'",
            "DELETE FROM tasks WHERE id = 't1'",
            "DELETE FROM repositories WHERE id = 'p1'",
        ] {
            assert!(exec(&pool, sql).await.is_err(), "{sql} was accepted");
        }
        exec(
            &pool,
            "DELETE FROM task_worktrees WHERE task_id = 't1' AND status = 'removed'",
        )
        .await
        .unwrap();
        exec(&pool, "DELETE FROM tasks WHERE id = 't1'")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn file_copies_reject_unsafe_relative_paths_and_follow_their_worktree() {
        let (pool, _dir) = with_worktree_task().await;
        exec(&pool, &worktree("k1", "t1", "p1", "w1", "ready"))
            .await
            .unwrap();
        let copy = |path: &str, size: i64| {
            format!("INSERT INTO task_worktree_file_copies VALUES ('k1', '{path}', {size}, 'x')")
        };

        for (index, sql) in [
            copy("", 1),
            copy("/etc/passwd", 1),
            copy("../secret", 1),
            copy("a/../../b", 1),
            copy("./a", 1),
            copy("a//b", 1),
            copy("a\\b", 1),
            copy("C:/x", 1),
            copy(".git/config", 1),
            copy("vendor/.GIT/config", 1),
            copy(".env", -1),
            copy(".env", 10_485_761),
            "INSERT INTO task_worktree_file_copies VALUES ('missing', '.env', 1, 'x')".to_string(),
        ]
        .iter()
        .enumerate()
        {
            assert!(exec(&pool, sql).await.is_err(), "case {index} was accepted");
        }
        for sql in [
            copy(".env", 12),
            copy("config/.env.local", 0),
            copy(".gitignore.local", 10_485_760),
        ] {
            exec(&pool, &sql).await.unwrap();
        }
        assert!(exec(&pool, &copy(".env", 12)).await.is_err());

        exec(&pool, "DELETE FROM task_worktrees WHERE id = 'k1'")
            .await
            .unwrap();
        assert_eq!(count(&pool, "task_worktree_file_copies").await, 0);
    }

    #[tokio::test]
    async fn deleting_a_workspace_removes_its_worktree_records() {
        let (pool, _dir) = with_worktree_task().await;
        exec(&pool, &worktree("k1", "t1", "p1", "w1", "ready"))
            .await
            .unwrap();
        exec(
            &pool,
            "INSERT INTO task_worktree_file_copies VALUES ('k1', '.env', 1, 'x')",
        )
        .await
        .unwrap();

        exec(&pool, "DELETE FROM workspaces WHERE id = 'w1'")
            .await
            .unwrap();

        assert_eq!(count(&pool, "task_worktrees").await, 0);
        assert_eq!(count(&pool, "task_worktree_file_copies").await, 0);
    }

    #[tokio::test]
    async fn the_profile_keeps_an_optional_worktrees_root() {
        let (pool, _dir) = seeded().await;
        exec(
            &pool,
            "INSERT INTO profile (id, locale, current_workspace_id, created_at, updated_at) VALUES ('singleton', 'en', 'w1', 'x', 'x')",
        )
        .await
        .unwrap();

        let root: Option<String> = sqlx::query_scalar("SELECT worktrees_root FROM profile")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(root, None);
        assert!(
            exec(&pool, "UPDATE profile SET worktrees_root = ''")
                .await
                .is_err()
        );
        exec(
            &pool,
            "UPDATE profile SET worktrees_root = '/Users/ana/Meelting'",
        )
        .await
        .unwrap();
    }
}
