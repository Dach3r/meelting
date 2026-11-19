use crate::domain::worktree::Environment;
use crate::repositories::task_worktrees;
use crate::services::session_load::{self, SessionError};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool) -> Result<Vec<Environment>, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    Ok(task_worktrees::environments_in_workspace(pool, &workspace_id).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::worktree_prepare::tests::{fixture, prepare, task};
    use crate::services::worktree_trash::tests::trashed;

    #[tokio::test]
    async fn lists_active_and_trashed_environments_with_their_task_and_repository() {
        let fixture = fixture().await;
        let active = task(&fixture, "Active", "main").await;
        prepare(&fixture, &active).await;
        let (trashed_id, _) = trashed(&fixture, "In the trash").await;
        task(&fixture, "Without environment", "main").await;

        let list = run(&fixture.pool).await.unwrap();

        let summary: Vec<(&str, &str, bool)> = list
            .iter()
            .map(|environment| {
                (
                    environment.task_title.as_str(),
                    environment.repository_name.as_str(),
                    environment.worktree.trashed_at.is_some(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [("In the trash", "app", true), ("Active", "app", false)]
        );
        assert_eq!(list[0].task_id, trashed_id);
    }
}
