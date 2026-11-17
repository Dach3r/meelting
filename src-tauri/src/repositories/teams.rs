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

pub async fn insert<'e>(
    executor: impl SqliteExecutor<'e>,
    id: &str,
    workspace_id: &str,
    name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO teams (id, workspace_id, name, created_at, updated_at) \
         VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .bind(id)
    .bind(workspace_id)
    .bind(name)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn exists<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
    team_id: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM teams WHERE id = ? AND workspace_id = ?)")
        .bind(team_id)
        .bind(workspace_id)
        .fetch_one(executor)
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

    #[tokio::test]
    async fn inserts_a_team_and_checks_it_belongs_to_the_workspace() {
        let (pool, _dir) = open_for_test().await;
        sqlx::query("INSERT INTO workspaces VALUES ('w1', 'A', 'x', 'x'), ('w2', 'B', 'x', 'x')")
            .execute(&pool)
            .await
            .unwrap();

        insert(&pool, "t1", "w1", "General").await.unwrap();

        assert!(exists(&pool, "w1", "t1").await.unwrap());
        assert!(!exists(&pool, "w2", "t1").await.unwrap());
        assert!(!exists(&pool, "w1", "missing").await.unwrap());
        assert!(insert(&pool, "t2", "w1", "  ").await.is_err());
    }
}
