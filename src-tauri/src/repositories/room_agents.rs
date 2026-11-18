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
         VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .bind(room_id)
    .bind(agent_id)
    .bind(workspace_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn list<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
) -> Result<Vec<(String, Participant)>, sqlx::Error> {
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT room_agents.room_id, agents.id, agents.name, agents.role FROM room_agents \
         JOIN agents ON agents.id = room_agents.agent_id \
         WHERE room_agents.workspace_id = ? ORDER BY agents.name, agents.id",
    )
    .bind(workspace_id)
    .fetch_all(executor)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(room_id, id, name, role)| (room_id, Participant { id, name, role }))
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
            "INSERT INTO rooms VALUES ('r1', 'w1', 'Sala', 'x', 'x'), ('r2', 'w2', 'Otra', 'x', 'x')",
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

    #[tokio::test]
    async fn rejects_an_agent_twice_in_the_same_room() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;
        insert(&pool, "r1", "a1", "w1").await.unwrap();

        assert!(insert(&pool, "r1", "a1", "w1").await.is_err());
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
