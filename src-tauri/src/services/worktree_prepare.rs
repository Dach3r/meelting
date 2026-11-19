use crate::domain::repository::RepositoryIdentity;
use crate::domain::storage::{StorageIssue, is_inside};
use crate::domain::task::{Task, TaskStatus};
use crate::domain::worktree::{
    WorktreeErrorCode, WorktreeStatus, branch_name, git_supports_attribute_source, simplified,
    worktree_folder,
};
use crate::repositories::task_worktrees::{self, NewWorktree};
use crate::repositories::{git_repositories, tasks};
use crate::services::repository_locks::RepositoryLocks;
use crate::services::session_load::{self, SessionError};
use crate::services::{tasks_list, worktrees_root_get};
use crate::system::git::{self, GitError};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub fn git_error(error: GitError) -> SessionError {
    match error {
        GitError::Unavailable(_) => SessionError::GitUnavailable,
        GitError::Failed(_) => SessionError::RepositoryUnavailable,
    }
}

fn preparable(task: &Task) -> bool {
    !matches!(task.status, TaskStatus::Completed | TaskStatus::Canceled)
        && task.repository_id.is_some()
        && task.base_ref.is_some()
}

pub async fn check_repository(
    repository: &RepositoryIdentity,
    check_storage: &impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<(), SessionError> {
    let path = Path::new(&repository.path);
    let layout = git::layout(path).await.map_err(git_error)?;
    if layout.common_dir != repository.git_common_dir || layout.git_dir != layout.common_dir {
        return Err(SessionError::RepositoryMoved);
    }
    check_storage(path).await?;
    check_storage(Path::new(&repository.git_common_dir)).await?;
    Ok(())
}

async fn check_commit(path: &Path, commit: &str) -> Result<(), SessionError> {
    if !git_supports_attribute_source(&git::version(path).await.map_err(git_error)?) {
        return Err(SessionError::GitTooOld);
    }
    if git::has_submodules(path, commit).await.map_err(git_error)? {
        return Err(SessionError::RepositoryWithSubmodules);
    }
    for filter in git::filters_at(path, commit).await.map_err(git_error)? {
        if git::filter_is_executable(path, &filter)
            .await
            .map_err(git_error)?
        {
            return Err(SessionError::FiltersNotSupported);
        }
    }
    Ok(())
}

fn prepare_folders(
    root: &Path,
    repository: &RepositoryIdentity,
    title: &str,
    worktree_id: &str,
) -> Result<PathBuf, SessionError> {
    std::fs::create_dir_all(root).map_err(|_| SessionError::InvalidWorktreesRoot)?;
    let root = std::fs::canonicalize(root)
        .map(simplified)
        .map_err(|_| SessionError::InvalidWorktreesRoot)?;
    let case_sensitive = cfg!(target_os = "linux");
    for location in [&repository.path, &repository.git_common_dir] {
        let location = std::fs::canonicalize(location)
            .map(simplified)
            .unwrap_or_else(|_| PathBuf::from(location));
        if is_inside(&root, std::slice::from_ref(&location), case_sensitive)
            || is_inside(&location, std::slice::from_ref(&root), case_sensitive)
        {
            return Err(SessionError::InvalidWorktreesRoot);
        }
    }
    let (repository_folder, folder) =
        worktree_folder(&root, &repository.name, &repository.id, title, worktree_id);
    match std::fs::symlink_metadata(&repository_folder) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Err(SessionError::WorktreePathConflict),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(&repository_folder)
                .map_err(|_| SessionError::WorktreePathConflict)?;
        }
        Err(_) => return Err(SessionError::WorktreePathConflict),
    }
    if std::fs::symlink_metadata(&folder).is_ok() {
        return Err(SessionError::WorktreePathConflict);
    }
    Ok(folder)
}

pub async fn run(
    pool: &SqlitePool,
    locks: &RepositoryLocks,
    task_id: &str,
    check_storage: impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<Task, SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let task = tasks_list::one(pool, &workspace_id, task_id).await?;
    if task.worktree.is_some() {
        return Err(SessionError::EnvironmentExists);
    }
    let (true, Some(repository_id), Some(base_ref)) =
        (preparable(&task), &task.repository_id, &task.base_ref)
    else {
        return Err(SessionError::TaskNotPreparable);
    };
    let repository = git_repositories::identity(pool, repository_id)
        .await?
        .ok_or(SessionError::RepositoryNotFound)?;
    let repository_path = Path::new(&repository.path);
    check_repository(&repository, &check_storage).await?;
    let commit = git::branch_commit(repository_path, base_ref)
        .await
        .map_err(git_error)?
        .ok_or(SessionError::InvalidBaseRef)?;
    check_commit(repository_path, &commit).await?;
    let (root, _) = worktrees_root_get::resolve(pool).await?;
    check_storage(&root).await?;
    let id = Uuid::now_v7().to_string();
    let folder = prepare_folders(&root, &repository, &task.title, &id)?;
    let folder_text = folder.to_string_lossy().into_owned();
    let branch = branch_name(&task.title, &id);

    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let current = tasks::get(&mut *transaction, &workspace_id, task_id)
        .await?
        .ok_or(SessionError::TaskNotFound)?;
    if !preparable(&current)
        || current.repository_id != task.repository_id
        || current.base_ref != task.base_ref
    {
        return Err(SessionError::TaskNotPreparable);
    }
    task_worktrees::insert_preparing(
        &mut *transaction,
        &NewWorktree {
            id: &id,
            workspace_id: &workspace_id,
            task_id,
            repository_id,
            path: &folder_text,
            branch_name: &branch,
            base_ref,
            base_commit_sha: &commit,
        },
    )
    .await
    .map_err(|error| match error.as_database_error() {
        Some(database) if database.is_unique_violation() => SessionError::EnvironmentExists,
        _ => SessionError::Database(error),
    })?;
    transaction.commit().await?;

    let _lock = locks.acquire(&repository.git_common_dir).await;
    let outcome = create(repository_path, &repository, &folder, &branch, &commit).await;
    let (status, error_code) = match outcome {
        Ok(()) => (WorktreeStatus::Ready, None),
        Err(code) => (WorktreeStatus::Failed, Some(code)),
    };
    task_worktrees::transition(pool, &id, WorktreeStatus::Preparing, status, error_code).await?;
    if error_code.is_some() {
        return Err(SessionError::EnvironmentFailed);
    }
    tasks_list::one(pool, &workspace_id, task_id).await
}

async fn create(
    repository_path: &Path,
    repository: &RepositoryIdentity,
    folder: &Path,
    branch: &str,
    commit: &str,
) -> Result<(), WorktreeErrorCode> {
    let conflict = std::fs::symlink_metadata(folder).is_ok()
        || git::branch_exists(repository_path, branch)
            .await
            .unwrap_or(true);
    if conflict
        || git::add_worktree(repository_path, folder, branch, commit)
            .await
            .is_err()
    {
        return Err(WorktreeErrorCode::CheckoutFailed);
    }
    if verify(repository, folder, branch, commit).await {
        Ok(())
    } else {
        Err(WorktreeErrorCode::VerificationFailed)
    }
}

pub async fn verify(
    repository: &RepositoryIdentity,
    folder: &Path,
    branch: &str,
    commit: &str,
) -> bool {
    let Ok(layout) = git::layout(folder).await else {
        return false;
    };
    let same_folder =
        std::fs::canonicalize(&layout.toplevel).ok() == std::fs::canonicalize(folder).ok();
    let head = git::head(folder).await.ok();
    let current_branch = git::symbolic_ref(folder, "HEAD").await.ok().flatten();
    same_folder
        && layout.common_dir == repository.git_common_dir
        && head.as_deref() == Some(commit)
        && current_branch == Some(format!("refs/heads/{branch}"))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::domain::locale::Locale;
    use crate::domain::task::{TaskDraft, TaskPriority};
    use crate::repositories::{database::open_for_test, profile};
    use crate::services::repository_register::tests::{git, local, repo_with_commit};
    use crate::services::{
        first_launch, repository_register, task_delete, task_save, task_status_change,
        worktree_discard, worktrees_reconcile,
    };

    pub struct Fixture {
        pub pool: SqlitePool,
        pub locks: RepositoryLocks,
        pub repo: PathBuf,
        pub root: PathBuf,
        pub repository_id: String,
        _dirs: Vec<tempfile::TempDir>,
    }

    pub async fn fixture() -> Fixture {
        let (pool, db) = open_for_test().await;
        first_launch::run(&pool, Locale::En, "Acme").await.unwrap();
        let parent = repo_with_commit("app");
        let repo = parent.path().join("app");
        std::fs::write(repo.join("README.md"), "hello\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-m", "readme"]);
        let root_dir = tempfile::tempdir().unwrap();
        let root = simplified(std::fs::canonicalize(root_dir.path()).unwrap());
        profile::set_worktrees_root(&pool, &root.to_string_lossy())
            .await
            .unwrap();
        let repository_id = repository_register::run(&pool, &repo, local).await.unwrap()[0]
            .id
            .clone();
        Fixture {
            pool,
            locks: RepositoryLocks::default(),
            repo,
            root,
            repository_id,
            _dirs: vec![db, parent, root_dir],
        }
    }

    pub async fn task(fixture: &Fixture, title: &str, base_ref: &str) -> Task {
        let draft = TaskDraft {
            title: title.into(),
            description: String::new(),
            priority: TaskPriority::Medium,
            repository_id: Some(fixture.repository_id.clone()),
            base_ref: Some(base_ref.into()),
            agent_ids: Vec::new(),
        };
        task_save::create(&fixture.pool, draft, None).await.unwrap()
    }

    fn rev(repo: &Path, reference: &str) -> String {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["rev-parse", reference])
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn status(repo: &Path) -> String {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["status", "--porcelain", "--ignored"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    async fn rows(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM task_worktrees")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    pub async fn prepare(fixture: &Fixture, task: &Task) -> Task {
        run(&fixture.pool, &fixture.locks, &task.id, local)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn prepares_a_worktree_from_the_exact_commit_without_touching_the_repository() {
        let fixture = fixture().await;
        let task = task(&fixture, "Fix the login", "main").await;
        std::fs::write(fixture.repo.join("draft.txt"), "uncommitted").unwrap();
        let before = status(&fixture.repo);

        let prepared = prepare(&fixture, &task).await;

        let worktree = prepared.worktree.unwrap();
        assert_eq!(worktree.status, WorktreeStatus::Ready);
        assert_eq!(worktree.base_commit_sha, rev(&fixture.repo, "main"));
        assert!(worktree.branch_name.starts_with("meelting/fix-the-login-"));
        let path = PathBuf::from(&worktree.path);
        assert!(path.starts_with(&fixture.root));
        assert!(path.join("README.md").exists());
        assert!(!path.join("draft.txt").exists());
        assert_eq!(rev(&path, "HEAD"), worktree.base_commit_sha);
        assert_eq!(status(&fixture.repo), before);
        assert_eq!(
            rev(&fixture.repo, "HEAD"),
            rev(&fixture.repo, "main"),
            "the original checkout did not move"
        );
    }

    #[tokio::test]
    async fn two_tasks_of_the_same_repository_get_independent_worktrees() {
        let fixture = fixture().await;
        let first = prepare(&fixture, &task(&fixture, "One", "main").await).await;
        let second = prepare(&fixture, &task(&fixture, "One", "main").await).await;

        let (first, second) = (first.worktree.unwrap(), second.worktree.unwrap());
        assert_ne!(first.path, second.path);
        assert_ne!(first.branch_name, second.branch_name);
        std::fs::write(PathBuf::from(&first.path).join("a.txt"), "a").unwrap();
        assert!(!PathBuf::from(&second.path).join("a.txt").exists());
    }

    #[tokio::test]
    async fn concurrent_requests_create_a_single_environment() {
        let fixture = fixture().await;
        let task = task(&fixture, "Once", "main").await;

        let (left, right) = tokio::join!(
            run(&fixture.pool, &fixture.locks, &task.id, local),
            run(&fixture.pool, &fixture.locks, &task.id, local)
        );

        assert_eq!(
            [left.is_ok(), right.is_ok()]
                .iter()
                .filter(|ok| **ok)
                .count(),
            1
        );
        assert!(matches!(
            left.err().or(right.err()),
            Some(SessionError::EnvironmentExists)
        ));
        assert_eq!(rows(&fixture.pool).await, 1);
        assert!(matches!(
            run(&fixture.pool, &fixture.locks, &task.id, local).await,
            Err(SessionError::EnvironmentExists)
        ));
    }

    #[tokio::test]
    async fn uses_the_chosen_base_branch_and_keeps_its_commit_when_it_moves() {
        let fixture = fixture().await;
        git(&fixture.repo, &["branch", "develop"]);
        let develop = rev(&fixture.repo, "develop");
        git(
            &fixture.repo,
            &["commit", "--allow-empty", "-m", "main moves"],
        );
        let task = task(&fixture, "From develop", "develop").await;

        let worktree = prepare(&fixture, &task).await.worktree.unwrap();
        git(&fixture.repo, &["checkout", "-q", "develop"]);
        git(
            &fixture.repo,
            &["commit", "--allow-empty", "-m", "develop moves"],
        );
        git(&fixture.repo, &["checkout", "-q", "main"]);

        assert_eq!(worktree.base_commit_sha, develop);
        assert_eq!(worktree.base_ref, "develop");
        let reloaded = tasks_list::one(&fixture.pool, &workspace(&fixture).await, &task.id)
            .await
            .unwrap();
        assert_eq!(reloaded.worktree.unwrap().base_commit_sha, develop);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn never_runs_repository_hooks() {
        use std::os::unix::fs::PermissionsExt;
        let fixture = fixture().await;
        let marker = fixture.root.join("hook-ran");
        let hook = fixture.repo.join(".git/hooks/post-checkout");
        std::fs::write(&hook, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();

        prepare(&fixture, &task(&fixture, "Hooks", "main").await).await;

        assert!(!marker.exists());
    }

    #[tokio::test]
    async fn rejects_executable_checkout_filters_but_not_unconfigured_attributes() {
        let fixture = fixture().await;
        std::fs::write(fixture.repo.join(".gitattributes"), "*.bin filter=evil\n").unwrap();
        std::fs::write(fixture.repo.join("data.bin"), "x").unwrap();
        git(&fixture.repo, &["add", ".gitattributes", "data.bin"]);
        git(&fixture.repo, &["commit", "-m", "attributes"]);
        let plain = task(&fixture, "Plain", "main").await;
        prepare(&fixture, &plain).await;

        git(
            &fixture.repo,
            &["config", "filter.evil.smudge", "touch pwned"],
        );
        let filtered = task(&fixture, "Filtered", "main").await;

        assert!(matches!(
            run(&fixture.pool, &fixture.locks, &filtered.id, local).await,
            Err(SessionError::FiltersNotSupported)
        ));
        assert_eq!(rows(&fixture.pool).await, 1);
    }

    #[tokio::test]
    async fn rejects_tasks_that_cannot_be_prepared_and_unsupported_storage() {
        let fixture = fixture().await;
        let completed = task(&fixture, "Done", "main").await;
        task_status_change::run(&fixture.pool, &completed.id, TaskStatus::Completed)
            .await
            .unwrap();
        let general = task_save::create(
            &fixture.pool,
            TaskDraft {
                title: "General".into(),
                description: String::new(),
                priority: TaskPriority::Low,
                repository_id: None,
                base_ref: None,
                agent_ids: Vec::new(),
            },
            None,
        )
        .await
        .unwrap();
        let open = task(&fixture, "Open", "main").await;
        let removable = async |_: &Path| Err(StorageIssue::Removable);

        for task_id in [&completed.id, &general.id] {
            assert!(matches!(
                run(&fixture.pool, &fixture.locks, task_id, local).await,
                Err(SessionError::TaskNotPreparable)
            ));
        }
        assert!(matches!(
            run(&fixture.pool, &fixture.locks, &open.id, removable).await,
            Err(SessionError::StorageRemovable)
        ));
        assert_eq!(rows(&fixture.pool).await, 0);
    }

    #[tokio::test]
    async fn completing_keeps_the_environment_and_it_blocks_deletion_and_repository_changes() {
        let fixture = fixture().await;
        let task = task(&fixture, "Keep", "main").await;
        let path = PathBuf::from(prepare(&fixture, &task).await.worktree.unwrap().path);

        let completed = task_status_change::run(&fixture.pool, &task.id, TaskStatus::Completed)
            .await
            .unwrap();

        assert_eq!(completed.worktree.unwrap().status, WorktreeStatus::Ready);
        assert!(path.exists());
        assert!(matches!(
            task_delete::run(&fixture.pool, &task.id).await,
            Err(SessionError::TaskHasEnvironment)
        ));
        let draft = TaskDraft {
            title: "Keep".into(),
            description: String::new(),
            priority: TaskPriority::Medium,
            repository_id: None,
            base_ref: None,
            agent_ids: Vec::new(),
        };
        assert!(matches!(
            task_save::update(&fixture.pool, &task.id, draft).await,
            Err(SessionError::TaskRepositoryLocked)
        ));
    }

    #[tokio::test]
    async fn a_missing_environment_is_detected_and_discarded_only_after_git_forgets_it() {
        let fixture = fixture().await;
        let task = task(&fixture, "Gone", "main").await;
        let worktree = prepare(&fixture, &task).await.worktree.unwrap();
        std::fs::remove_dir_all(&worktree.path).unwrap();

        worktrees_reconcile::run(&fixture.pool, &fixture.locks)
            .await
            .unwrap();
        let reloaded = tasks_list::one(&fixture.pool, &workspace(&fixture).await, &task.id)
            .await
            .unwrap();
        assert_eq!(reloaded.worktree.unwrap().status, WorktreeStatus::Missing);
        assert!(matches!(
            worktree_discard::run(&fixture.pool, &fixture.locks, &task.id).await,
            Err(SessionError::EnvironmentStillExists)
        ));

        git(&fixture.repo, &["worktree", "prune"]);
        let discarded = worktree_discard::run(&fixture.pool, &fixture.locks, &task.id)
            .await
            .unwrap();

        assert!(discarded.worktree.is_none());
        assert!(!rev(&fixture.repo, &worktree.branch_name).is_empty());
        prepare(&fixture, &task).await;
        assert_eq!(rows(&fixture.pool).await, 2);
        task_status_change::run(&fixture.pool, &task.id, TaskStatus::InProgress)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn interrupted_preparations_are_recovered_or_marked_failed_without_deleting() {
        let fixture = fixture().await;
        let workspace_id = workspace(&fixture).await;
        let sha = rev(&fixture.repo, "main");
        let finished = task(&fixture, "Finished", "main").await;
        let unfinished = task(&fixture, "Unfinished", "main").await;
        let done_path = fixture.root.join("done");
        let lost_path = fixture.root.join("lost");
        for (id, task, path, branch) in [
            ("w-done", &finished, &done_path, "meelting/done-1"),
            ("w-lost", &unfinished, &lost_path, "meelting/lost-1"),
        ] {
            task_worktrees::insert_preparing(
                &fixture.pool,
                &NewWorktree {
                    id,
                    workspace_id: &workspace_id,
                    task_id: &task.id,
                    repository_id: &fixture.repository_id,
                    path: &path.to_string_lossy(),
                    branch_name: branch,
                    base_ref: "main",
                    base_commit_sha: &sha,
                },
            )
            .await
            .unwrap();
        }
        git(
            &fixture.repo,
            &[
                "worktree",
                "add",
                "-b",
                "meelting/done-1",
                done_path.to_str().unwrap(),
                &sha,
            ],
        );

        worktrees_reconcile::run(&fixture.pool, &fixture.locks)
            .await
            .unwrap();

        let done = tasks_list::one(&fixture.pool, &workspace_id, &finished.id)
            .await
            .unwrap()
            .worktree
            .unwrap();
        let lost = tasks_list::one(&fixture.pool, &workspace_id, &unfinished.id)
            .await
            .unwrap()
            .worktree
            .unwrap();
        assert_eq!(done.status, WorktreeStatus::Ready);
        assert_eq!(lost.status, WorktreeStatus::Failed);
        assert_eq!(
            lost.error_code,
            Some(WorktreeErrorCode::PreparationInterrupted)
        );
        assert!(done_path.exists());
        let discarded = worktree_discard::run(&fixture.pool, &fixture.locks, &unfinished.id)
            .await
            .unwrap();
        assert!(discarded.worktree.is_none());
    }

    #[tokio::test]
    async fn deleting_a_task_removes_only_its_removed_history() {
        let fixture = fixture().await;
        let task = task(&fixture, "History", "main").await;
        let worktree = prepare(&fixture, &task).await.worktree.unwrap();
        std::fs::remove_dir_all(&worktree.path).unwrap();
        git(&fixture.repo, &["worktree", "prune"]);
        worktrees_reconcile::run(&fixture.pool, &fixture.locks)
            .await
            .unwrap();
        worktree_discard::run(&fixture.pool, &fixture.locks, &task.id)
            .await
            .unwrap();

        task_delete::run(&fixture.pool, &task.id).await.unwrap();

        assert_eq!(rows(&fixture.pool).await, 0);
    }

    async fn workspace(fixture: &Fixture) -> String {
        session_load::require(&fixture.pool)
            .await
            .unwrap()
            .current_workspace_id
    }
}
