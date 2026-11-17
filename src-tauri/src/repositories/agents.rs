use crate::domain::team::{Agent, AgentDraft};
use sqlx::SqliteExecutor;

pub async fn list<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
) -> Result<Vec<Agent>, sqlx::Error> {
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT id, team_id, name, role FROM agents WHERE workspace_id = ? ORDER BY created_at, id",
    )
    .bind(workspace_id)
    .fetch_all(executor)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, team_id, name, role)| Agent {
            id,
            team_id,
            name,
            role,
        })
        .collect())
}

pub async fn insert<'e>(
    executor: impl SqliteExecutor<'e>,
    id: &str,
    workspace_id: &str,
    draft: &AgentDraft,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO agents (id, workspace_id, team_id, name, role, description, instructions, provider, model, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
    )
    .bind(id)
    .bind(workspace_id)
    .bind(&draft.team_id)
    .bind(&draft.name)
    .bind(&draft.role)
    .bind(&draft.description)
    .bind(&draft.instructions)
    .bind(&draft.provider)
    .bind(&draft.model)
    .execute(executor)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::database::open_for_test;

    #[tokio::test]
    async fn lists_only_the_agents_of_the_workspace() {
        let (pool, _dir) = open_for_test().await;
        for sql in [
            "INSERT INTO workspaces VALUES ('w1', 'A', 'x', 'x'), ('w2', 'B', 'x', 'x')",
            "INSERT INTO teams VALUES ('t1', 'w1', 'General', 'x', 'x'), ('t2', 'w2', 'Other', 'x', 'x')",
            "INSERT INTO agents VALUES ('a1', 'w1', 't1', NULL, 'Ana', 'Developer', '', '', 'openai', 'gpt', 'x', 'x'), ('a2', 'w2', 't2', NULL, 'Bob', 'QA', '', '', 'openai', 'gpt', 'x', 'x')",
        ] {
            sqlx::query(sql).execute(&pool).await.unwrap();
        }

        let agents = list(&pool, "w1").await.unwrap();

        assert_eq!(
            agents,
            [Agent {
                id: "a1".into(),
                team_id: "t1".into(),
                name: "Ana".into(),
                role: "Developer".into()
            }]
        );
    }

    #[tokio::test]
    async fn inserts_a_custom_agent_without_template() {
        let (pool, _dir) = open_for_test().await;
        sqlx::query("INSERT INTO workspaces VALUES ('w1', 'A', 'x', 'x')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO teams VALUES ('t1', 'w1', 'General', 'x', 'x')")
            .execute(&pool)
            .await
            .unwrap();
        let draft = AgentDraft {
            team_id: "t1".into(),
            name: "Ana".into(),
            role: "Developer".into(),
            description: String::new(),
            instructions: String::new(),
            provider: "openai".into(),
            model: "gpt".into(),
        };

        insert(&pool, "a1", "w1", &draft).await.unwrap();

        let template_key: Option<String> =
            sqlx::query_scalar("SELECT template_key FROM agents WHERE id = 'a1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(template_key, None);
        assert_eq!(list(&pool, "w1").await.unwrap().len(), 1);
    }
}
