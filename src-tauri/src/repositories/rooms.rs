use sqlx::SqliteExecutor;

pub type RoomRow = (String, String, String, Option<String>);

pub async fn list<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
) -> Result<Vec<RoomRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, name, updated_at, finished_at FROM rooms WHERE workspace_id = ? \
         ORDER BY finished_at IS NOT NULL, updated_at DESC, id DESC",
    )
    .bind(workspace_id)
    .fetch_all(executor)
    .await
}

pub async fn get<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
    id: &str,
) -> Result<Option<RoomRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id, name, updated_at, finished_at FROM rooms \
         WHERE id = ? AND workspace_id = ? AND finished_at IS NULL",
    )
    .bind(id)
    .bind(workspace_id)
    .fetch_optional(executor)
    .await
}

pub async fn workspace_id<'e>(
    executor: impl SqliteExecutor<'e>,
    id: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT workspace_id FROM rooms WHERE id = ?")
        .bind(id)
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

pub async fn finish<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
    id: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE rooms SET finished_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ? AND workspace_id = ? AND finished_at IS NULL",
    )
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
            "INSERT INTO rooms VALUES ('r1', 'w1', 'Old', '1', '1', NULL), ('r2', 'w1', 'New', '1', '3', NULL), ('r3', 'w1', 'Tie', '1', '3', NULL), ('r4', 'w2', 'Other', '1', '9', NULL)",
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
            .map(|(id, ..)| id)
            .collect();

        assert_eq!(ids, ["r3", "r2", "r1"]);
    }

    #[tokio::test]
    async fn lists_finished_rooms_last_with_their_finish_time() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;
        finish(&pool, "w1", "r3").await.unwrap();

        let rows = list(&pool, "w1").await.unwrap();

        let ids: Vec<&str> = rows.iter().map(|(id, ..)| id.as_str()).collect();
        assert_eq!(ids, ["r2", "r1", "r3"]);
        assert!(rows[0].3.is_none());
        assert!(rows[2].3.is_some());
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
    async fn reads_and_finishes_only_rooms_of_the_workspace() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;

        assert_eq!(get(&pool, "w1", "r4").await.unwrap(), None);
        assert!(!finish(&pool, "w1", "r4").await.unwrap());
        assert!(finish(&pool, "w1", "r1").await.unwrap());
        assert!(!finish(&pool, "w1", "r1").await.unwrap());
        assert_eq!(get(&pool, "w1", "r1").await.unwrap(), None);
        assert!(get(&pool, "w2", "r4").await.unwrap().is_some());
        assert_eq!(
            workspace_id(&pool, "r4").await.unwrap().as_deref(),
            Some("w2")
        );
        assert_eq!(
            workspace_id(&pool, "r1").await.unwrap().as_deref(),
            Some("w1")
        );
    }
}
