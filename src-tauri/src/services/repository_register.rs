use crate::domain::repository::{Repository, name_from_path};
use crate::domain::storage::StorageIssue;
use crate::repositories::git_repositories;
use crate::services::repositories_list;
use crate::services::session_load::{self, SessionError};
use crate::system::git::{self, GitError};
use sqlx::SqlitePool;
use std::path::Path;
use uuid::Uuid;

fn inspection_error(error: GitError) -> SessionError {
    match error {
        GitError::Unavailable(_) => SessionError::GitUnavailable,
        GitError::Failed(_) => SessionError::NotARepository,
    }
}

pub async fn run(
    pool: &SqlitePool,
    folder: &Path,
    check_storage: impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<Vec<Repository>, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    if git::is_bare(folder).await.map_err(inspection_error)? {
        return Err(SessionError::RepositoryBare);
    }
    let layout = git::layout(folder).await.map_err(inspection_error)?;
    if layout.git_dir != layout.common_dir {
        return Err(SessionError::RepositoryIsWorktree);
    }
    let toplevel = Path::new(&layout.toplevel);
    if !git::has_commits(toplevel).await.map_err(inspection_error)? {
        return Err(SessionError::RepositoryWithoutCommits);
    }
    if git::has_submodules(toplevel)
        .await
        .map_err(inspection_error)?
    {
        return Err(SessionError::RepositoryWithSubmodules);
    }
    check_storage(toplevel).await?;
    check_storage(Path::new(&layout.common_dir)).await?;
    git_repositories::insert(
        pool,
        &Uuid::now_v7().to_string(),
        &workspace_id,
        &name_from_path(&layout.toplevel),
        &layout.toplevel,
        &layout.common_dir,
    )
    .await
    .map_err(|error| match error.as_database_error() {
        Some(database) if database.is_unique_violation() => {
            SessionError::RepositoryAlreadyRegistered
        }
        _ => SessionError::Database(error),
    })?;
    repositories_list::run(pool).await
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::repositories::database::open_for_test;
    use crate::services::{first_launch, workspace_create};
    use std::process::Command;

    pub async fn local(_: &Path) -> Result<(), StorageIssue> {
        Ok(())
    }

    pub fn git(path: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .status()
            .unwrap();
        assert!(status.success());
    }

    pub fn repo_with_commit(name: &str) -> tempfile::TempDir {
        let parent = tempfile::tempdir().unwrap();
        let repo = parent.path().join(name);
        std::fs::create_dir(&repo).unwrap();
        git(&repo, &["init", "--initial-branch=main"]);
        git(&repo, &["commit", "--allow-empty", "-m", "init"]);
        parent
    }

    async fn count(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM repositories")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn registers_the_top_level_folder_of_a_repository() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let parent = repo_with_commit("my app ñ");
        let repo = parent.path().join("my app ñ");
        let nested = repo.join("src");
        std::fs::create_dir(&nested).unwrap();

        let list = run(&pool, &nested, local).await.unwrap();

        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "my app ñ");
        assert_eq!(
            std::fs::canonicalize(&list[0].path).unwrap(),
            std::fs::canonicalize(&repo).unwrap()
        );
    }

    #[tokio::test]
    async fn rejects_the_same_repository_twice_in_a_workspace_but_not_in_another() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let parent = repo_with_commit("app");
        let repo = parent.path().join("app");
        run(&pool, &repo, local).await.unwrap();

        assert!(matches!(
            run(&pool, &repo.join("."), local).await,
            Err(SessionError::RepositoryAlreadyRegistered)
        ));
        workspace_create::run(&pool, "Globex").await.unwrap();
        assert_eq!(run(&pool, &repo, local).await.unwrap().len(), 1);
        assert_eq!(count(&pool).await, 2);
    }

    #[tokio::test]
    async fn rejects_unsupported_folders_without_writing() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let plain = tempfile::tempdir().unwrap();
        let bare = tempfile::tempdir().unwrap();
        git(bare.path(), &["init", "--bare"]);
        let empty = tempfile::tempdir().unwrap();
        git(empty.path(), &["init"]);
        let parent = repo_with_commit("app");
        let repo = parent.path().join("app");
        let worktree = parent.path().join("linked");
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                "linked",
                worktree.to_str().unwrap(),
            ],
        );
        let modules = repo_with_commit("modules");
        let modules = modules.path().join("modules");
        std::fs::write(modules.join(".gitmodules"), "").unwrap();
        git(&modules, &["add", ".gitmodules"]);
        git(&modules, &["commit", "-m", "modules"]);

        assert!(matches!(
            run(&pool, plain.path(), local).await,
            Err(SessionError::NotARepository)
        ));
        assert!(matches!(
            run(&pool, &plain.path().join("missing"), local).await,
            Err(SessionError::NotARepository)
        ));
        assert!(matches!(
            run(&pool, bare.path(), local).await,
            Err(SessionError::RepositoryBare)
        ));
        assert!(matches!(
            run(&pool, empty.path(), local).await,
            Err(SessionError::RepositoryWithoutCommits)
        ));
        assert!(matches!(
            run(&pool, &worktree, local).await,
            Err(SessionError::RepositoryIsWorktree)
        ));
        assert!(matches!(
            run(&pool, &modules, local).await,
            Err(SessionError::RepositoryWithSubmodules)
        ));
        assert_eq!(count(&pool).await, 0);
    }

    #[tokio::test]
    async fn rejects_repositories_on_unsupported_storage_without_writing() {
        let (pool, _dir) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let parent = repo_with_commit("app");
        let synced = async |_: &Path| Err(StorageIssue::Synced);

        assert!(matches!(
            run(&pool, &parent.path().join("app"), synced).await,
            Err(SessionError::StorageSynced)
        ));
        assert_eq!(count(&pool).await, 0);
    }
}
