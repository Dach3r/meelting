use crate::domain::team::Position;
use sqlx::SqliteExecutor;

pub async fn list<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
) -> Result<Vec<Position>, sqlx::Error> {
    let rows: Vec<(String, f64, f64)> =
        sqlx::query_as("SELECT node_id, x, y FROM canvas_positions WHERE workspace_id = ?")
            .bind(workspace_id)
            .fetch_all(executor)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(node_id, x, y)| Position { node_id, x, y })
        .collect())
}

pub async fn save<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
    position: &Position,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO canvas_positions (workspace_id, node_id, x, y) VALUES (?, ?, ?, ?) \
         ON CONFLICT (workspace_id, node_id) DO UPDATE SET x = excluded.x, y = excluded.y",
    )
    .bind(workspace_id)
    .bind(&position.node_id)
    .bind(position.x)
    .bind(position.y)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn delete<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
    node_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM canvas_positions WHERE workspace_id = ? AND node_id = ?")
        .bind(workspace_id)
        .bind(node_id)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn clear<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM canvas_positions WHERE workspace_id = ?")
        .bind(workspace_id)
        .execute(executor)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::database::open_for_test;

    fn position(node_id: &str, x: f64, y: f64) -> Position {
        Position {
            node_id: node_id.into(),
            x,
            y,
        }
    }

    #[tokio::test]
    async fn saves_overwrites_and_deletes_positions_per_workspace() {
        let (pool, _dir) = open_for_test().await;
        sqlx::query("INSERT INTO workspaces VALUES ('w1', 'A', 'x', 'x'), ('w2', 'B', 'x', 'x')")
            .execute(&pool)
            .await
            .unwrap();

        save(&pool, "w1", &position("n1", 10.0, 20.0))
            .await
            .unwrap();
        save(&pool, "w1", &position("n1", -5.5, 7.0)).await.unwrap();
        save(&pool, "w2", &position("n2", 1.0, 2.0)).await.unwrap();

        assert_eq!(
            list(&pool, "w1").await.unwrap(),
            [position("n1", -5.5, 7.0)]
        );
        delete(&pool, "w1", "n1").await.unwrap();
        assert!(list(&pool, "w1").await.unwrap().is_empty());
        assert_eq!(list(&pool, "w2").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn clears_only_the_positions_of_the_workspace() {
        let (pool, _dir) = open_for_test().await;
        sqlx::query("INSERT INTO workspaces VALUES ('w1', 'A', 'x', 'x'), ('w2', 'B', 'x', 'x')")
            .execute(&pool)
            .await
            .unwrap();
        save(&pool, "w1", &position("n1", 1.0, 2.0)).await.unwrap();
        save(&pool, "w1", &position("n2", 3.0, 4.0)).await.unwrap();
        save(&pool, "w2", &position("n3", 5.0, 6.0)).await.unwrap();

        clear(&pool, "w1").await.unwrap();

        assert!(list(&pool, "w1").await.unwrap().is_empty());
        assert_eq!(list(&pool, "w2").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn removes_positions_with_their_workspace() {
        let (pool, _dir) = open_for_test().await;
        sqlx::query("INSERT INTO workspaces VALUES ('w1', 'A', 'x', 'x')")
            .execute(&pool)
            .await
            .unwrap();
        save(&pool, "w1", &position("n1", 1.0, 2.0)).await.unwrap();

        sqlx::query("DELETE FROM workspaces WHERE id = 'w1'")
            .execute(&pool)
            .await
            .unwrap();

        assert!(list(&pool, "w1").await.unwrap().is_empty());
    }
}
