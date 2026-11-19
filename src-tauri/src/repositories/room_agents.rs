use crate::domain::room::Participant;
use sqlx::SqliteExecutor;

pub async fn insert<'e>(
    executor: impl SqliteExecutor<'e>,
    room_id: &str,
    agent_id: &str,
    workspace_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO room_agents (room_id, agent_id, workspace_id, created_at) \
         VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) \
         ON CONFLICT (room_id, agent_id) DO UPDATE SET left_at = NULL",
    )
    .bind(room_id)
    .bind(agent_id)
    .bind(workspace_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn leave<'e>(
    executor: impl SqliteExecutor<'e>,
    room_id: &str,
    agent_id: &str,
    workspace_id: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE room_agents SET left_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE room_id = ? AND agent_id = ? AND workspace_id = ? AND left_at IS NULL",
    )
    .bind(room_id)
    .bind(agent_id)
    .bind(workspace_id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn list<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
) -> Result<Vec<(String, Participant)>, sqlx::Error> {
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT room_agents.room_id, agents.id, agents.name, agents.role FROM room_agents \
         JOIN agents ON agents.id = room_agents.agent_id \
         WHERE room_agents.workspace_id = ? AND room_agents.left_at IS NULL \
         ORDER BY agents.name, agents.id",
    )
    .bind(workspace_id)
    .fetch_all(executor)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(room_id, id, name, role)| (room_id, Participant { id, name, role }))
        .collect())
}

pub async fn former<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
    room_id: &str,
) -> Result<Vec<Participant>, sqlx::Error> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT agents.id, agents.name, agents.role FROM room_agents \
         JOIN agents ON agents.id = room_agents.agent_id \
         WHERE room_agents.workspace_id = ? AND room_agents.room_id = ? \
         AND room_agents.left_at IS NOT NULL ORDER BY agents.name, agents.id",
    )
    .bind(workspace_id)
    .bind(room_id)
    .fetch_all(executor)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, role)| Participant { id, name, role })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::database::open_for_test;

    async fn seed(pool: &sqlx::SqlitePool) {
        for sql in [
            "INSERT INTO workspaces VALUES ('w1', 'A', 'x', 'x'), ('w2', 'B', 'x', 'x')",
            "INSERT INTO teams VALUES ('t1', 'w1', 'General', 'x', 'x'), ('t2', 'w2', 'Other', 'x', 'x')",
            "INSERT INTO agents VALUES ('a1', 'w1', 't1', NULL, 'Bea', 'QA', '', '', 'openai', 'gpt', 'x', 'x'), ('a2', 'w1', 't1', NULL, 'Ana', 'Dev', '', '', 'openai', 'gpt', 'x', 'x'), ('a3', 'w2', 't2', NULL, 'Eva', 'QA', '', '', 'openai', 'gpt', 'x', 'x')",
            "INSERT INTO rooms VALUES ('r1', 'w1', 'Sala', 'x', 'x', NULL), ('r2', 'w2', 'Otra', 'x', 'x', NULL)",
        ] {
            sqlx::query(sql).execute(pool).await.unwrap();
        }
    }

    #[tokio::test]
    async fn lists_the_participants_of_the_workspace_by_name() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;
        insert(&pool, "r1", "a1", "w1").await.unwrap();
        insert(&pool, "r1", "a2", "w1").await.unwrap();
        insert(&pool, "r2", "a3", "w2").await.unwrap();

        let names: Vec<(String, String)> = list(&pool, "w1")
            .await
            .unwrap()
            .into_iter()
            .map(|(room_id, agent)| (room_id, agent.name))
            .collect();

        assert_eq!(
            names,
            [("r1".into(), "Ana".into()), ("r1".into(), "Bea".into())]
        );
    }

    async fn names(pool: &sqlx::SqlitePool) -> Vec<String> {
        list(pool, "w1")
            .await
            .unwrap()
            .into_iter()
            .map(|(_, agent)| agent.name)
            .collect()
    }

    #[tokio::test]
    async fn keeps_one_participation_when_an_agent_is_added_twice() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;
        insert(&pool, "r1", "a1", "w1").await.unwrap();
        insert(&pool, "r1", "a1", "w1").await.unwrap();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM room_agents")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn leaving_keeps_the_participation_as_former() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;
        insert(&pool, "r1", "a1", "w1").await.unwrap();
        insert(&pool, "r1", "a2", "w1").await.unwrap();

        assert!(leave(&pool, "r1", "a1", "w1").await.unwrap());
        assert!(!leave(&pool, "r1", "a1", "w1").await.unwrap());

        assert_eq!(names(&pool).await, ["Ana"]);
        let former: Vec<String> = former(&pool, "w1", "r1")
            .await
            .unwrap()
            .into_iter()
            .map(|agent| agent.name)
            .collect();
        assert_eq!(former, ["Bea"]);
    }

    #[tokio::test]
    async fn rejoining_clears_the_departure() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;
        insert(&pool, "r1", "a1", "w1").await.unwrap();
        leave(&pool, "r1", "a1", "w1").await.unwrap();

        insert(&pool, "r1", "a1", "w1").await.unwrap();

        assert_eq!(names(&pool).await, ["Bea"]);
        assert!(former(&pool, "w1", "r1").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn does_not_leave_rooms_of_another_workspace() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;
        insert(&pool, "r2", "a3", "w2").await.unwrap();

        assert!(!leave(&pool, "r2", "a3", "w1").await.unwrap());
        assert!(former(&pool, "w1", "r2").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_a_departure_before_joining() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;
        insert(&pool, "r1", "a1", "w1").await.unwrap();

        let result = sqlx::query("UPDATE room_agents SET left_at = '2000-01-01T00:00:00.000Z'")
            .execute(&pool)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_rooms_and_agents_of_another_workspace() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;

        assert!(insert(&pool, "r1", "a3", "w1").await.is_err());
        assert!(insert(&pool, "r1", "a3", "w2").await.is_err());
        assert!(insert(&pool, "r2", "a1", "w1").await.is_err());
        assert!(list(&pool, "w1").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn leaves_the_room_when_the_agent_without_messages_is_deleted() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;
        insert(&pool, "r1", "a1", "w1").await.unwrap();

        sqlx::query("DELETE FROM agents WHERE id = 'a1'")
            .execute(&pool)
            .await
            .unwrap();

        assert!(list(&pool, "w1").await.unwrap().is_empty());
    }
}
