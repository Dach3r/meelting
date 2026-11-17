use sqlx::SqliteExecutor;

pub async fn list<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as("SELECT id, name FROM teams WHERE workspace_id = ? ORDER BY created_at, id")
        .bind(workspace_id)
        .fetch_all(executor)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::database::open_for_test;

    #[tokio::test]
    async fn lists_only_the_teams_of_the_workspace() {
        let (pool, _dir) = open_for_test().await;
        for sql in [
            "INSERT INTO workspaces VALUES ('w1', 'A', 'x', 'x'), ('w2', 'B', 'x', 'x')",
            "INSERT INTO teams VALUES ('t2', 'w1', 'Design', 'y', 'y'), ('t1', 'w1', 'General', 'x', 'x'), ('t3', 'w2', 'Other', 'x', 'x')",
        ] {
            sqlx::query(sql).execute(&pool).await.unwrap();
        }

        let teams = list(&pool, "w1").await.unwrap();

        assert_eq!(
            teams,
            [
                ("t1".to_string(), "General".to_string()),
                ("t2".to_string(), "Design".to_string())
            ]
        );
    }
}
