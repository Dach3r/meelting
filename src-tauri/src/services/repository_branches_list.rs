use crate::domain::repository::{RepositoryBranches, default_branch};
use crate::repositories::git_repositories;
use crate::services::session_load::{self, SessionError};
use crate::system::git::{self, GitError};
use sqlx::SqlitePool;
use std::path::Path;

fn unavailable(error: GitError) -> SessionError {
    match error {
        GitError::Unavailable(_) => SessionError::GitUnavailable,
        GitError::Failed(_) => SessionError::RepositoryUnavailable,
    }
}

pub async fn run(
    pool: &SqlitePool,
    repository_id: &str,
) -> Result<RepositoryBranches, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let repository = git_repositories::get(pool, &workspace_id, repository_id)
        .await?
        .ok_or(SessionError::RepositoryNotFound)?;
    let path = Path::new(&repository.path);
    let branches = git::local_branches(path).await.map_err(unavailable)?;
    let origin_head = git::symbolic_ref(path, "refs/remotes/origin/HEAD")
        .await
        .map_err(unavailable)?;
    let head = git::symbolic_ref(path, "HEAD").await.map_err(unavailable)?;
    let default_branch = default_branch(origin_head.as_deref(), head.as_deref(), &branches);
    Ok(RepositoryBranches {
        branches,
        default_branch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::first_launch;
    use crate::services::repository_register::{self, tests::git, tests::repo_with_commit};

    #[tokio::test]
    async fn lists_the_local_branches_and_suggests_the_current_one() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let parent = repo_with_commit("app");
        let repo = parent.path().join("app");
        git(&repo, &["branch", "feature/login"]);
        let id = repository_register::run(&pool, &repo).await.unwrap()[0]
            .id
            .clone();

        let result = run(&pool, &id).await.unwrap();

        assert_eq!(result.branches, ["feature/login", "main"]);
        assert_eq!(result.default_branch.as_deref(), Some("main"));
    }

    #[tokio::test]
    async fn reports_a_repository_that_is_no_longer_on_disk() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let parent = repo_with_commit("app");
        let repo = parent.path().join("app");
        let id = repository_register::run(&pool, &repo).await.unwrap()[0]
            .id
            .clone();
        std::fs::remove_dir_all(&repo).unwrap();

        assert!(matches!(
            run(&pool, &id).await,
            Err(SessionError::RepositoryUnavailable)
        ));
        assert!(matches!(
            run(&pool, "missing").await,
            Err(SessionError::RepositoryNotFound)
        ));
    }
}
