use crate::repositories::agent_runs;
use sqlx::SqlitePool;

pub async fn run(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    agent_runs::expire_active(pool).await
}
