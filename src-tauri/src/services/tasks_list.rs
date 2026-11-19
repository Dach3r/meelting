use crate::domain::task::Task;
use crate::repositories::{task_agents, tasks};
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool) -> Result<Vec<Task>, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let mut transaction = pool.begin().await?;
    let mut list = tasks::list(&mut *transaction, &workspace_id).await?;
    let assignments = task_agents::list(&mut *transaction, &workspace_id).await?;
    transaction.commit().await?;
    for task in &mut list {
        with_agents(task, &assignments);
    }
    Ok(list)
}

pub async fn one(pool: &SqlitePool, workspace_id: &str, id: &str) -> Result<Task, SessionError> {
    let mut transaction = pool.begin().await?;
    let mut task = tasks::get(&mut *transaction, workspace_id, id)
        .await?
        .ok_or(SessionError::TaskNotFound)?;
    let assignments = task_agents::list(&mut *transaction, workspace_id).await?;
    transaction.commit().await?;
    with_agents(&mut task, &assignments);
    Ok(task)
}

fn with_agents(task: &mut Task, assignments: &[(String, String)]) {
    task.agent_ids = assignments
        .iter()
        .filter(|(task_id, _)| *task_id == task.id)
        .map(|(_, agent_id)| agent_id.clone())
        .collect();
}
