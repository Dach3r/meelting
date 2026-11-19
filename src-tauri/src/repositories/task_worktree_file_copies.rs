use crate::domain::file_copy::FileCopy;
use sqlx::SqliteExecutor;

pub async fn active_in_workspace<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
) -> Result<Vec<(String, FileCopy)>, sqlx::Error> {
    let rows: Vec<(String, String, i64, String)> = sqlx::query_as(
        "SELECT copies.task_worktree_id, copies.relative_path, copies.size_bytes, copies.copied_at \
         FROM task_worktree_file_copies AS copies \
         JOIN task_worktrees ON task_worktrees.id = copies.task_worktree_id \
         WHERE task_worktrees.workspace_id = ? AND task_worktrees.status <> 'removed' \
         ORDER BY copies.relative_path",
    )
    .bind(workspace_id)
    .fetch_all(executor)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(worktree_id, relative_path, size_bytes, copied_at)| {
            (
                worktree_id,
                FileCopy {
                    relative_path,
                    size_bytes: size_bytes.max(0) as u64,
                    copied_at,
                },
            )
        })
        .collect())
}

pub async fn record<'e>(
    executor: impl SqliteExecutor<'e>,
    task_worktree_id: &str,
    relative_path: &str,
    size_bytes: u64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO task_worktree_file_copies (task_worktree_id, relative_path, size_bytes, copied_at) \
         VALUES (?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) \
         ON CONFLICT (task_worktree_id, relative_path) \
         DO UPDATE SET size_bytes = excluded.size_bytes, copied_at = excluded.copied_at",
    )
    .bind(task_worktree_id)
    .bind(relative_path)
    .bind(size_bytes as i64)
    .execute(executor)
    .await?;
    Ok(())
}
