use crate::domain::repository::{Repository, RepositoryIdentity};
use sqlx::SqliteExecutor;

pub async fn list<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
) -> Result<Vec<Repository>, sqlx::Error> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, name, path FROM repositories WHERE workspace_id = ? ORDER BY name, id",
    )
    .bind(workspace_id)
    .fetch_all(executor)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, path)| Repository { id, name, path })
        .collect())
}

pub async fn get<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
    id: &str,
) -> Result<Option<Repository>, sqlx::Error> {
    let row: Option<(String, String, String)> =
        sqlx::query_as("SELECT id, name, path FROM repositories WHERE id = ? AND workspace_id = ?")
            .bind(id)
            .bind(workspace_id)
            .fetch_optional(executor)
            .await?;
    Ok(row.map(|(id, name, path)| Repository { id, name, path }))
}

pub async fn insert<'e>(
    executor: impl SqliteExecutor<'e>,
    id: &str,
    workspace_id: &str,
    name: &str,
    path: &str,
    git_common_dir: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO repositories (id, workspace_id, name, path, git_common_dir, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .bind(id)
    .bind(workspace_id)
    .bind(name)
    .bind(path)
    .bind(git_common_dir)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn delete<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
    id: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM repositories WHERE id = ? AND workspace_id = ?")
        .bind(id)
        .bind(workspace_id)
        .execute(executor)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn identity<'e>(
    executor: impl SqliteExecutor<'e>,
    id: &str,
) -> Result<Option<RepositoryIdentity>, sqlx::Error> {
    let row: Option<(String, String, String, String)> =
        sqlx::query_as("SELECT id, name, path, git_common_dir FROM repositories WHERE id = ?")
            .bind(id)
            .fetch_optional(executor)
            .await?;
    Ok(
        row.map(|(id, name, path, git_common_dir)| RepositoryIdentity {
            id,
            name,
            path,
            git_common_dir,
        }),
    )
}

pub async fn all_locations<'e>(
    executor: impl SqliteExecutor<'e>,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as("SELECT path, git_common_dir FROM repositories")
        .fetch_all(executor)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::database::open_for_test;

    async fn exec(pool: &sqlx::SqlitePool, sql: &str) -> Result<(), sqlx::Error> {
        sqlx::query(sqlx::AssertSqlSafe(sql.to_string()))
            .execute(pool)
            .await
            .map(|_| ())
    }

    async fn seed(pool: &sqlx::SqlitePool) {
        exec(
            pool,
            "INSERT INTO workspaces VALUES ('w1', 'A', 'x', 'x'), ('w2', 'B', 'x', 'x')",
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn lists_the_repositories_of_the_workspace_by_name() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;
        insert(&pool, "r1", "w1", "zeta", "/z", "/z/.git")
            .await
            .unwrap();
        insert(&pool, "r2", "w1", "alpha", "/a", "/a/.git")
            .await
            .unwrap();
        insert(&pool, "r3", "w2", "beta", "/b", "/b/.git")
            .await
            .unwrap();

        let names: Vec<String> = list(&pool, "w1")
            .await
            .unwrap()
            .into_iter()
            .map(|repository| repository.name)
            .collect();

        assert_eq!(names, ["alpha", "zeta"]);
        assert_eq!(get(&pool, "w2", "r1").await.unwrap(), None);
    }

    #[tokio::test]
    async fn the_same_git_identity_is_unique_per_workspace_only() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;
        insert(&pool, "r1", "w1", "app", "/app", "/app/.git")
            .await
            .unwrap();

        let duplicate = insert(&pool, "r2", "w1", "app", "/app", "/app/.git").await;

        assert!(
            duplicate
                .unwrap_err()
                .as_database_error()
                .unwrap()
                .is_unique_violation()
        );
        insert(&pool, "r3", "w2", "app", "/app", "/app/.git")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn rejects_blank_and_overlong_names_and_empty_paths() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;

        assert!(
            insert(&pool, "r1", "w1", " ", "/a", "/a/.git")
                .await
                .is_err()
        );
        assert!(
            insert(&pool, "r1", "w1", &"a".repeat(81), "/a", "/a/.git")
                .await
                .is_err()
        );
        assert!(insert(&pool, "r1", "w1", "a", "", "/a/.git").await.is_err());
        assert!(insert(&pool, "r1", "w1", "a", "/a", "").await.is_err());
    }

    #[tokio::test]
    async fn deletes_only_inside_the_workspace() {
        let (pool, _dir) = open_for_test().await;
        seed(&pool).await;
        insert(&pool, "r1", "w1", "app", "/app", "/app/.git")
            .await
            .unwrap();

        assert!(!delete(&pool, "w2", "r1").await.unwrap());
        assert!(delete(&pool, "w1", "r1").await.unwrap());
        assert!(list(&pool, "w1").await.unwrap().is_empty());
    }
}
