use crate::domain::workspace::Workspace;
use sqlx::SqliteExecutor;

pub async fn insert<'e>(
    executor: impl SqliteExecutor<'e>,
    id: &str,
    name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO workspaces (id, name, created_at, updated_at) \
         VALUES (?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .bind(id)
    .bind(name)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn list<'e>(executor: impl SqliteExecutor<'e>) -> Result<Vec<Workspace>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT id, name FROM workspaces ORDER BY id")
        .fetch_all(executor)
        .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name)| Workspace { id, name })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::database::open_for_test;

    #[tokio::test]
    async fn lists_inserted_workspaces_in_creation_order() {
        let (pool, _dir) = open_for_test().await;
        insert(&pool, "01", "First").await.unwrap();
        insert(&pool, "02", "Second").await.unwrap();

        let workspaces = list(&pool).await.unwrap();

        assert_eq!(
            workspaces,
            [
                Workspace {
                    id: "01".into(),
                    name: "First".into()
                },
                Workspace {
                    id: "02".into(),
                    name: "Second".into()
                },
            ]
        );
    }

    #[tokio::test]
    async fn allows_repeated_names() {
        let (pool, _dir) = open_for_test().await;
        insert(&pool, "01", "Acme").await.unwrap();

        insert(&pool, "02", "Acme").await.unwrap();

        assert_eq!(list(&pool).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn database_rejects_blank_and_overlong_names() {
        let (pool, _dir) = open_for_test().await;

        assert!(insert(&pool, "01", "   ").await.is_err());
        assert!(insert(&pool, "02", &"a".repeat(81)).await.is_err());
    }
}
