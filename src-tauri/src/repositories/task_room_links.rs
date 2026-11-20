use crate::domain::task::TaskRoomLink;
use sqlx::SqliteExecutor;

#[derive(sqlx::FromRow)]
struct Row {
    task_id: String,
    room_id: String,
    room_name: String,
    message_id: Option<String>,
}

pub async fn list<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
) -> Result<Vec<(String, TaskRoomLink)>, sqlx::Error> {
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT task_room_links.task_id, task_room_links.room_id, rooms.name AS room_name, task_room_links.message_id \
         FROM task_room_links JOIN rooms ON rooms.id = task_room_links.room_id \
         WHERE task_room_links.workspace_id = ? \
         ORDER BY task_room_links.created_at, task_room_links.id",
    )
    .bind(workspace_id)
    .fetch_all(executor)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.task_id,
                TaskRoomLink {
                    room_id: row.room_id,
                    room_name: row.room_name,
                    message_id: row.message_id,
                },
            )
        })
        .collect())
}

pub async fn candidate_tasks<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
    room_id: &str,
    agent_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT DISTINCT tasks.id FROM task_room_links \
         JOIN tasks ON tasks.id = task_room_links.task_id AND tasks.workspace_id = task_room_links.workspace_id \
         JOIN task_agents ON task_agents.task_id = tasks.id AND task_agents.agent_id = ? \
         WHERE task_room_links.workspace_id = ? AND task_room_links.room_id = ? \
           AND tasks.status NOT IN ('completed', 'canceled') \
         ORDER BY tasks.id LIMIT 2",
    )
    .bind(agent_id)
    .bind(workspace_id)
    .bind(room_id)
    .fetch_all(executor)
    .await
}

pub async fn insert<'e>(
    executor: impl SqliteExecutor<'e>,
    id: &str,
    task_id: &str,
    workspace_id: &str,
    room_id: &str,
    message_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO task_room_links (id, task_id, room_id, message_id, workspace_id, created_at) \
         SELECT ?1, ?2, ?3, ?4, ?5, strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE NOT EXISTS (SELECT 1 FROM task_room_links WHERE task_id = ?2 AND room_id = ?3 AND message_id IS ?4)",
    )
    .bind(id)
    .bind(task_id)
    .bind(room_id)
    .bind(message_id)
    .bind(workspace_id)
    .execute(executor)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::database::open_for_test;

    async fn seeded() -> (sqlx::SqlitePool, tempfile::TempDir) {
        let (pool, dir) = open_for_test().await;
        for sql in [
            "INSERT INTO workspaces VALUES ('w1', 'A', 'x', 'x')",
            "INSERT INTO rooms (id, workspace_id, name, created_at, updated_at) VALUES ('r1', 'w1', 'Daily', 'x', 'x')",
            "INSERT INTO messages (id, room_id, agent_id, body, created_at) VALUES ('m1', 'r1', NULL, 'Hi', 'x')",
            "INSERT INTO tasks (id, workspace_id, title, description, status, priority, created_at, updated_at) VALUES ('k1', 'w1', 'T', '', 'pending', 'medium', 'x', 'x')",
        ] {
            sqlx::query(sql).execute(&pool).await.unwrap();
        }
        (pool, dir)
    }

    #[tokio::test]
    async fn inserts_once_per_room_and_message() {
        let (pool, _dir) = seeded().await;
        insert(&pool, "l1", "k1", "w1", "r1", None).await.unwrap();
        insert(&pool, "l2", "k1", "w1", "r1", None).await.unwrap();
        insert(&pool, "l3", "k1", "w1", "r1", Some("m1"))
            .await
            .unwrap();
        insert(&pool, "l4", "k1", "w1", "r1", Some("m1"))
            .await
            .unwrap();
        let links = list(&pool, "w1").await.unwrap();
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].1.room_name, "Daily");
        assert_eq!(links[0].1.message_id, None);
        assert_eq!(links[1].1.message_id.as_deref(), Some("m1"));
    }

    #[tokio::test]
    async fn rejects_a_message_from_another_room_and_cascades_with_the_task() {
        let (pool, _dir) = seeded().await;
        sqlx::query("INSERT INTO rooms (id, workspace_id, name, created_at, updated_at) VALUES ('r2', 'w1', 'Other', 'x', 'x')")
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            insert(&pool, "l1", "k1", "w1", "r2", Some("m1"))
                .await
                .is_err()
        );
        insert(&pool, "l2", "k1", "w1", "r1", None).await.unwrap();
        sqlx::query("DELETE FROM tasks WHERE id = 'k1'")
            .execute(&pool)
            .await
            .unwrap();
        assert!(list(&pool, "w1").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn candidates_are_the_open_tasks_of_the_room_assigned_to_the_agent() {
        let (pool, _dir) = seeded().await;
        for sql in [
            "INSERT INTO teams VALUES ('g1', 'w1', 'General', 'x', 'x')",
            "INSERT INTO agents (id, workspace_id, team_id, name, role, description, instructions, provider, model, created_at, updated_at) \
             VALUES ('a1', 'w1', 'g1', 'Ada', 'Dev', '', '', 'local', 'qwen', 'x', 'x'), ('a2', 'w1', 'g1', 'Bob', 'Dev', '', '', 'local', 'qwen', 'x', 'x')",
            "INSERT INTO tasks (id, workspace_id, title, description, status, priority, completed_at, created_at, updated_at) \
             VALUES ('k2', 'w1', 'T', '', 'completed', 'medium', 'x', 'x', 'x'), ('k3', 'w1', 'T', '', 'in_progress', 'medium', NULL, 'x', 'x'), ('k4', 'w1', 'T', '', 'pending', 'medium', NULL, 'x', 'x')",
            "INSERT INTO task_agents VALUES ('k1', 'a1', 'w1', 'x'), ('k2', 'a1', 'w1', 'x'), ('k3', 'a1', 'w1', 'x'), ('k4', 'a2', 'w1', 'x')",
        ] {
            sqlx::query(sql).execute(&pool).await.unwrap();
        }
        for (id, task) in [("l1", "k1"), ("l2", "k2"), ("l3", "k4")] {
            insert(&pool, id, task, "w1", "r1", None).await.unwrap();
        }

        let only = |agent: &'static str| candidate_tasks(&pool, "w1", "r1", agent);
        assert_eq!(only("a1").await.unwrap(), vec!["k1"]);
        assert_eq!(only("a2").await.unwrap(), vec!["k4"]);

        insert(&pool, "l4", "k3", "w1", "r1", Some("m1"))
            .await
            .unwrap();
        insert(&pool, "l5", "k3", "w1", "r1", None).await.unwrap();
        assert_eq!(only("a1").await.unwrap(), vec!["k1", "k3"]);
        assert!(
            candidate_tasks(&pool, "w1", "other", "a1")
                .await
                .unwrap()
                .is_empty()
        );
    }
}
