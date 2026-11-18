use sqlx::SqliteExecutor;

pub async fn list<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
) -> Result<Vec<(String, String, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, name, updated_at FROM rooms WHERE workspace_id = ? ORDER BY updated_at DESC, id DESC",
    )
    .bind(workspace_id)
    .fetch_all(executor)
    .await
}

pub async fn get<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
    id: &str,
) -> Result<Option<(String, String, String)>, sqlx::Error> {
    sqlx::query_as("SELECT id, name, updated_at FROM rooms WHERE id = ? AND workspace_id = ?")
        .bind(id)
        .bind(workspace_id)
        .fetch_optional(executor)
        .await
}

pub async fn insert<'e>(
    executor: impl SqliteExecutor<'e>,
    id: &str,
    workspace_id: &str,
    name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO rooms (id, workspace_id, name, created_at, updated_at) \
         VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .bind(id)
    .bind(workspace_id)
    .bind(name)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn touch<'e>(
    executor: impl SqliteExecutor<'e>,
    id: &str,
    updated_at: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE rooms SET updated_at = ? WHERE id = ?")
        .bind(updated_at)
        .bind(id)
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn delete<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
    id: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM rooms WHERE id = ? AND workspace_id = ?")
        .bind(id)
        .bind(workspace_id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::database::open_for_test;

    async fn seed(pool: &sqlx::SqlitePool) {
        for sql in [
            "INSERT INTO workspaces VALUES ('w1', 'A', 'x', 'x'), ('w2', 'B', 'x', 'x')",
            "INSERT INTO rooms VALUES ('r1', 'w1', 'Old', '1', '1'), ('r2', 'w1', 'New', '1', '3'), ('r3', 'w1', 'Tie', '1', '3'), ('r4', 'w2', 'Other', '1', '9')",
        ] {
            sqlx::query(sql).execute(pool).await.unwrap();
        }
    }

    #[tokio::test]
    async fn lists_the_rooms_of_the_workspace_by_recent_activity() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;

        let ids: Vec<String> = list(&pool, "w1")
            .await
            .unwrap()
            .into_iter()
            .map(|(id, _, _)| id)
            .collect();

        assert_eq!(ids, ["r3", "r2", "r1"]);
    }

    #[tokio::test]
    async fn starts_with_the_activity_at_the_creation_time() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;

        insert(&pool, "r9", "w1", "Fresh").await.unwrap();

        let (created_at, updated_at): (String, String) =
            sqlx::query_as("SELECT created_at, updated_at FROM rooms WHERE id = 'r9'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(created_at, updated_at);
    }

    #[tokio::test]
    async fn rejects_blank_and_long_names() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;

        assert!(insert(&pool, "r8", "w1", "   ").await.is_err());
        assert!(insert(&pool, "r9", "w1", &"a".repeat(81)).await.is_err());
    }

    #[tokio::test]
    async fn reads_and_deletes_only_rooms_of_the_workspace() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;

        assert_eq!(get(&pool, "w1", "r4").await.unwrap(), None);
        assert!(!delete(&pool, "w1", "r4").await.unwrap());
        assert!(delete(&pool, "w1", "r1").await.unwrap());
        assert_eq!(get(&pool, "w1", "r1").await.unwrap(), None);
        assert!(get(&pool, "w2", "r4").await.unwrap().is_some());
    }
}
