use crate::domain::worktree::{Worktree, WorktreeErrorCode, WorktreeStatus};
use sqlx::SqliteExecutor;

const COLUMNS: &str =
    "id, path, branch_name, base_ref, base_commit_sha, status, error_code, created_at, updated_at";
const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%fZ', 'now')";

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    owner_id: String,
    path: String,
    branch_name: String,
    base_ref: String,
    base_commit_sha: String,
    status: String,
    error_code: Option<String>,
    created_at: String,
    updated_at: String,
}

fn to_worktree(row: Row) -> Result<(String, Worktree), sqlx::Error> {
    let invalid = |column: &str| sqlx::Error::ColumnDecode {
        index: column.into(),
        source: "unknown value".into(),
    };
    let worktree = Worktree {
        status: WorktreeStatus::parse(&row.status).ok_or_else(|| invalid("status"))?,
        error_code: match row.error_code {
            Some(code) => {
                Some(WorktreeErrorCode::parse(&code).ok_or_else(|| invalid("error_code"))?)
            }
            None => None,
        },
        id: row.id,
        path: row.path,
        branch_name: row.branch_name,
        base_ref: row.base_ref,
        base_commit_sha: row.base_commit_sha,
        created_at: row.created_at,
        updated_at: row.updated_at,
    };
    Ok((row.owner_id, worktree))
}

pub async fn active_in_workspace<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
) -> Result<Vec<(String, Worktree)>, sqlx::Error> {
    let rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS}, task_id AS owner_id FROM task_worktrees WHERE workspace_id = ? AND status <> 'removed'"
    )))
    .bind(workspace_id)
    .fetch_all(executor)
    .await?;
    rows.into_iter().map(to_worktree).collect()
}

pub async fn active_for_task<'e>(
    executor: impl SqliteExecutor<'e>,
    workspace_id: &str,
    task_id: &str,
) -> Result<Option<Worktree>, sqlx::Error> {
    let row: Option<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS}, task_id AS owner_id FROM task_worktrees \
         WHERE workspace_id = ? AND task_id = ? AND status <> 'removed'"
    )))
    .bind(workspace_id)
    .bind(task_id)
    .fetch_optional(executor)
    .await?;
    Ok(row
        .map(to_worktree)
        .transpose()?
        .map(|(_, worktree)| worktree))
}

pub async fn transient<'e>(
    executor: impl SqliteExecutor<'e>,
) -> Result<Vec<(String, Worktree)>, sqlx::Error> {
    let rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {COLUMNS}, repository_id AS owner_id FROM task_worktrees \
         WHERE status IN ('preparing', 'ready', 'missing', 'removing')"
    )))
    .fetch_all(executor)
    .await?;
    rows.into_iter().map(to_worktree).collect()
}

pub async fn has_history<'e>(
    executor: impl SqliteExecutor<'e>,
    task_id: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM task_worktrees WHERE task_id = ?)")
        .bind(task_id)
        .fetch_one(executor)
        .await
}

pub struct NewWorktree<'a> {
    pub id: &'a str,
    pub workspace_id: &'a str,
    pub task_id: &'a str,
    pub repository_id: &'a str,
    pub path: &'a str,
    pub branch_name: &'a str,
    pub base_ref: &'a str,
    pub base_commit_sha: &'a str,
}

pub async fn insert_preparing<'e>(
    executor: impl SqliteExecutor<'e>,
    worktree: &NewWorktree<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "INSERT INTO task_worktrees (id, workspace_id, task_id, repository_id, path, branch_name, base_ref, base_commit_sha, status, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'preparing', {NOW}, {NOW})"
    )))
    .bind(worktree.id)
    .bind(worktree.workspace_id)
    .bind(worktree.task_id)
    .bind(worktree.repository_id)
    .bind(worktree.path)
    .bind(worktree.branch_name)
    .bind(worktree.base_ref)
    .bind(worktree.base_commit_sha)
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn transition<'e>(
    executor: impl SqliteExecutor<'e>,
    id: &str,
    from: WorktreeStatus,
    to: WorktreeStatus,
    error_code: Option<WorktreeErrorCode>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE task_worktrees SET status = ?1, \
         error_code = CASE WHEN ?1 = 'removed' THEN COALESCE(?2, error_code) ELSE ?2 END, \
         removed_at = CASE WHEN ?1 = 'removed' THEN {NOW} END, updated_at = {NOW} \
         WHERE id = ?3 AND status = ?4"
    )))
    .bind(to.as_str())
    .bind(error_code.map(WorktreeErrorCode::as_str))
    .bind(id)
    .bind(from.as_str())
    .execute(executor)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_removed<'e>(
    executor: impl SqliteExecutor<'e>,
    task_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM task_worktrees WHERE task_id = ? AND status = 'removed'")
        .bind(task_id)
        .execute(executor)
        .await?;
    Ok(())
}
