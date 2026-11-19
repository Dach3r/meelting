use sqlx::SqliteExecutor;

pub async fn list<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT task_agents.task_id, task_agents.agent_id FROM task_agents \
         JOIN agents ON agents.id = task_agents.agent_id \
         WHERE task_agents.workspace_id = ? ORDER BY agents.name, agents.id",
    )
    .bind(workspace_id)
    .fetch_all(executor)
    .await
}

pub async fn insert<'e>(
    executor: impl SqliteExecutor<'e>,
    task_id: &str,
    agent_id: &str,
    workspace_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO task_agents (task_id, agent_id, workspace_id, created_at) \
         VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) \
         ON CONFLICT (task_id, agent_id) DO NOTHING",
    )
    .bind(task_id)
    .bind(agent_id)
    .bind(workspace_id)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn delete_others<'e>(
    executor: impl SqliteExecutor<'e>,
    task_id: &str,
    kept_agent_ids: &[String],
) -> Result<(), sqlx::Error> {
    let kept =
        serde_json::to_string(kept_agent_ids).map_err(|error| sqlx::Error::Encode(error.into()))?;
    sqlx::query(
        "DELETE FROM task_agents WHERE task_id = ? \
         AND agent_id NOT IN (SELECT value FROM json_each(?))",
    )
    .bind(task_id)
    .bind(kept)
    .execute(executor)
    .await?;
    Ok(())
}
