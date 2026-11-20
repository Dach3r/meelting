use crate::repositories::{agent_runs, tool_calls};
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    tool_calls::cancel_all_running(pool).await?;
    agent_runs::expire_active(pool).await
}
