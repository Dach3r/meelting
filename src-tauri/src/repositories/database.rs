use sqlx::SqlitePool;
use sqlx::migrate::MigrateError;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Migrate(#[from] MigrateError),
}

pub async fn open(path: &Path) -> Result<SqlitePool, DatabaseError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new().connect_with(options).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

#[cfg(test)]
pub async fn open_for_test() -> (SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open(&dir.path().join("test.db")).await.unwrap();
    (pool, dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn applies_migrations_and_enables_foreign_keys() {
        let (pool, _dir) = open_for_test().await;

        let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&pool)
            .await
            .unwrap();
        let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .unwrap();
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('agents', 'profile', 'teams', 'workspaces') ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(foreign_keys, 1);
        assert_eq!(journal_mode, "wal");
        assert_eq!(tables, ["agents", "profile", "teams", "workspaces"]);
    }

    #[tokio::test]
    async fn reopening_an_existing_database_keeps_its_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let pool = open(&path).await.unwrap();
        sqlx::query(
            "INSERT INTO workspaces (id, name, created_at, updated_at) VALUES ('a', 'Acme', 'x', 'x')",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        let reopened = open(&path).await.unwrap();
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM workspaces")
            .fetch_one(&reopened)
            .await
            .unwrap();

        assert_eq!(count, 1);
    }

    async fn exec(pool: &SqlitePool, sql: &str) -> Result<(), sqlx::Error> {
        sqlx::query(sqlx::AssertSqlSafe(sql.to_string()))
            .execute(pool)
            .await
            .map(|_| ())
    }

    async fn seeded_teams() -> (SqlitePool, tempfile::TempDir) {
        let (pool, dir) = open_for_test().await;
        for sql in [
            "INSERT INTO workspaces VALUES ('w1', 'A', 'x', 'x'), ('w2', 'B', 'x', 'x')",
            "INSERT INTO teams VALUES ('t1', 'w1', 'General', 'x', 'x'), ('t2', 'w2', 'Other', 'x', 'x')",
        ] {
            exec(&pool, sql).await.unwrap();
        }
        (pool, dir)
    }

    fn agent_sql(id: &str, team: &str, workspace: &str, overrides: (&str, &str)) -> String {
        let mut columns = [
            ("name", "'Ada'".to_string()),
            ("role", "'Dev'".to_string()),
            ("description", "''".to_string()),
            ("instructions", "''".to_string()),
            ("provider", "'local'".to_string()),
            ("model", "'qwen'".to_string()),
        ];
        for (column, value) in columns.iter_mut() {
            if *column == overrides.0 {
                *value = overrides.1.to_string();
            }
        }
        format!(
            "INSERT INTO agents (id, workspace_id, team_id, template_key, name, role, description, instructions, provider, model, created_at, updated_at) \
             VALUES ('{id}', '{workspace}', '{team}', NULL, {}, {}, {}, {}, {}, {}, 'x', 'x')",
            columns[0].1, columns[1].1, columns[2].1, columns[3].1, columns[4].1, columns[5].1
        )
    }

    #[tokio::test]
    async fn teams_reject_blank_and_overlong_names_and_unknown_workspaces() {
        let (pool, _dir) = seeded_teams().await;

        assert!(
            exec(
                &pool,
                "INSERT INTO teams VALUES ('t3', 'w1', '   ', 'x', 'x')"
            )
            .await
            .is_err()
        );
        let overlong = format!(
            "INSERT INTO teams VALUES ('t3', 'w1', '{}', 'x', 'x')",
            "a".repeat(81)
        );
        assert!(exec(&pool, &overlong).await.is_err());
        assert!(
            exec(
                &pool,
                "INSERT INTO teams VALUES ('t3', 'missing', 'Team', 'x', 'x')"
            )
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn agents_enforce_text_limits() {
        let (pool, _dir) = seeded_teams().await;
        let cases = [
            ("name", "'   '".to_string()),
            ("name", format!("'{}'", "a".repeat(81))),
            ("role", "''".to_string()),
            ("role", format!("'{}'", "a".repeat(81))),
            ("description", format!("'{}'", "a".repeat(501))),
            ("instructions", format!("'{}'", "a".repeat(10001))),
            ("provider", "' '".to_string()),
            ("model", "''".to_string()),
        ];

        for (index, (column, value)) in cases.iter().enumerate() {
            let sql = agent_sql(&format!("a{index}"), "t1", "w1", (column, value));
            assert!(
                exec(&pool, &sql).await.is_err(),
                "{column} case {index} was accepted"
            );
        }
    }

    #[tokio::test]
    async fn agents_accept_text_at_the_limits() {
        let (pool, _dir) = seeded_teams().await;
        for (index, (column, value)) in [
            ("name", format!("'{}'", "a".repeat(80))),
            ("description", format!("'{}'", "a".repeat(500))),
            ("instructions", format!("'{}'", "a".repeat(10000))),
        ]
        .iter()
        .enumerate()
        {
            let sql = agent_sql(&format!("a{index}"), "t1", "w1", (column, value));
            exec(&pool, &sql).await.unwrap();
        }
    }

    #[tokio::test]
    async fn agents_cannot_belong_to_a_team_of_another_workspace() {
        let (pool, _dir) = seeded_teams().await;

        assert!(
            exec(&pool, &agent_sql("a1", "t2", "w1", ("name", "'Ada'")))
                .await
                .is_err()
        );
        exec(&pool, &agent_sql("a2", "t1", "w1", ("name", "'Ada'")))
            .await
            .unwrap();
        assert!(
            exec(&pool, "UPDATE agents SET team_id = 't2' WHERE id = 'a2'")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn a_team_with_agents_cannot_be_deleted() {
        let (pool, _dir) = seeded_teams().await;
        exec(&pool, &agent_sql("a1", "t1", "w1", ("name", "'Ada'")))
            .await
            .unwrap();

        assert!(
            exec(&pool, "DELETE FROM teams WHERE id = 't1'")
                .await
                .is_err()
        );

        exec(&pool, "DELETE FROM agents WHERE id = 'a1'")
            .await
            .unwrap();
        exec(&pool, "DELETE FROM teams WHERE id = 't1'")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn deleting_a_workspace_removes_its_teams_and_agents() {
        let (pool, _dir) = seeded_teams().await;
        exec(&pool, &agent_sql("a1", "t1", "w1", ("name", "'Ada'")))
            .await
            .unwrap();

        exec(&pool, "DELETE FROM workspaces WHERE id = 'w1'")
            .await
            .unwrap();

        let teams: i64 = sqlx::query_scalar("SELECT count(*) FROM teams WHERE workspace_id = 'w1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        let agents: i64 =
            sqlx::query_scalar("SELECT count(*) FROM agents WHERE workspace_id = 'w1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!((teams, agents), (0, 0));
    }
}
