use crate::domain::repository::RepositoryIdentity;
use crate::domain::storage::StorageIssue;
use crate::domain::task::Task;
use crate::domain::worktree::{
    ChangeKind, IN_PROGRESS_MARKERS, PendingChange, RemovalCheck, Worktree, WorktreeErrorCode,
    WorktreeStatus, parse_status,
};
use crate::repositories::{git_repositories, task_worktrees};
use crate::services::repository_locks::RepositoryLocks;
use crate::services::session_load::{self, SessionError};
use crate::services::tasks_list;
use crate::services::worktree_prepare::{check_repository, git_error};
use crate::services::worktrees_reconcile::{gone, intact};
use crate::system::git;
use crate::system::git_guarded::{Guard, GuardError};
use sqlx::SqlitePool;
use std::path::Path;

pub async fn ready_environment(
    pool: &SqlitePool,
    task_id: &str,
) -> Result<(String, Worktree, RepositoryIdentity), SessionError> {
    let workspace_id = session_load::require(pool).await?.current_workspace_id;
    let task = tasks_list::one(pool, &workspace_id, task_id).await?;
    let worktree = task
        .worktree
        .filter(|worktree| worktree.status == WorktreeStatus::Ready)
        .ok_or(SessionError::EnvironmentNotFound)?;
    if worktree.trashed_at.is_some() {
        return Err(SessionError::EnvironmentInTrash);
    }
    let repository_id = task
        .repository_id
        .ok_or(SessionError::EnvironmentNotFound)?;
    let repository = git_repositories::identity(pool, &repository_id)
        .await?
        .ok_or(SessionError::RepositoryNotFound)?;
    Ok((workspace_id, worktree, repository))
}

pub async fn verify_identity(
    repository: &RepositoryIdentity,
    worktree: &Worktree,
    check_storage: &impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<(), SessionError> {
    check_repository(repository, check_storage).await?;
    let path = Path::new(worktree.location());
    check_storage(path).await?;
    let entry = intact(repository, worktree)
        .await
        .ok_or(SessionError::EnvironmentIdentityMismatch)?;
    let on_branch = git::symbolic_ref(path, "HEAD").await.map_err(git_error)?;
    if entry.branch.as_deref() != Some(worktree.branch_name.as_str())
        || on_branch != Some(format!("refs/heads/{}", worktree.branch_name))
    {
        return Err(SessionError::EnvironmentIdentityMismatch);
    }
    Ok(())
}

fn guard_error(error: GuardError) -> SessionError {
    match error {
        GuardError::Unavailable(_) => SessionError::GitUnavailable,
        GuardError::GitTooOld => SessionError::GitTooOld,
        _ => SessionError::EnvironmentCheckRefused,
    }
}

async fn inspect(
    repository: &RepositoryIdentity,
    worktree: &Worktree,
    check_storage: &impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<RemovalCheck, SessionError> {
    verify_identity(repository, worktree, check_storage).await?;
    let guard = Guard::open(
        repository,
        Path::new(&worktree.path),
        &worktree.base_commit_sha,
    )
    .await
    .map_err(guard_error)?;
    let expected = format!("refs/heads/{}", worktree.branch_name);
    if guard.head_reference().await.map_err(guard_error)? != Some(expected) {
        return Err(SessionError::EnvironmentIdentityMismatch);
    }
    let mut changes = parse_status(&guard.status_exhaustive().await.map_err(guard_error)?);
    changes.extend(
        guard
            .gitlinks()
            .await
            .map_err(guard_error)?
            .into_iter()
            .map(|path| PendingChange {
                kind: ChangeKind::Tracked,
                path,
            }),
    );
    let mut operation_in_progress = false;
    for marker in IN_PROGRESS_MARKERS {
        let marker = guard.git_path(marker).await.map_err(guard_error)?;
        operation_in_progress |= std::fs::symlink_metadata(marker).is_ok();
    }
    let commits_ahead = guard
        .commits_ahead(&worktree.branch_name)
        .await
        .map_err(guard_error)?;
    guard.finish().await.map_err(guard_error)?;
    Ok(RemovalCheck {
        changes,
        operation_in_progress,
        commits_ahead,
    })
}

pub async fn check(
    pool: &SqlitePool,
    locks: &RepositoryLocks,
    task_id: &str,
    check_storage: impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<RemovalCheck, SessionError> {
    let (_, worktree, repository) = ready_environment(pool, task_id).await?;
    let _lock = locks.acquire(&repository.git_common_dir).await;
    inspect(&repository, &worktree, &check_storage).await
}

pub async fn run(
    pool: &SqlitePool,
    locks: &RepositoryLocks,
    task_id: &str,
    check_storage: impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<Task, SessionError> {
    let (workspace_id, worktree, repository) = ready_environment(pool, task_id).await?;
    let _lock = locks.acquire(&repository.git_common_dir).await;
    let inspection = inspect(&repository, &worktree, &check_storage).await?;
    if !inspection.changes.is_empty() {
        return Err(SessionError::EnvironmentHasChanges);
    }
    if inspection.operation_in_progress {
        return Err(SessionError::EnvironmentOperationInProgress);
    }
    use WorktreeStatus::*;
    if !task_worktrees::transition(pool, &worktree.id, Ready, Removing, None).await? {
        return Err(SessionError::EnvironmentNotFound);
    }
    let removed = git::remove_worktree(Path::new(&repository.path), Path::new(&worktree.path))
        .await
        .is_ok();
    let (status, error_code) = if gone(&repository, &worktree).await {
        (Removed, None)
    } else if intact(&repository, &worktree).await.is_some() {
        (Ready, None)
    } else {
        (Failed, Some(WorktreeErrorCode::RemovalInterrupted))
    };
    task_worktrees::transition(pool, &worktree.id, Removing, status, error_code).await?;
    if !removed || status != Removed {
        return Err(SessionError::EnvironmentRemoveRefused);
    }
    tasks_list::one(pool, &workspace_id, task_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::worktree::{ChangeKind, PendingChange};
    use crate::services::repository_register::tests::{git, local};
    use crate::services::task_delete;
    use crate::services::worktree_prepare::tests::{fixture, prepare, task};
    use std::path::PathBuf;

    fn branch_exists(repo: &Path, branch: &str) -> bool {
        std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args([
                "show-ref",
                "--verify",
                "--quiet",
                &format!("refs/heads/{branch}"),
            ])
            .status()
            .unwrap()
            .success()
    }

    fn index_lock(worktree: &Path) -> PathBuf {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(worktree)
            .args([
                "rev-parse",
                "--path-format=absolute",
                "--git-path",
                "index.lock",
            ])
            .output()
            .unwrap();
        PathBuf::from(String::from_utf8_lossy(&output.stdout).trim_end())
    }

    #[tokio::test]
    async fn removes_a_clean_environment_and_keeps_its_branch_and_commits() {
        let fixture = fixture().await;
        let task = task(&fixture, "Done", "main").await;
        let worktree = prepare(&fixture, &task).await.worktree.unwrap();
        let path = PathBuf::from(&worktree.path);
        std::fs::write(path.join("feature.txt"), "work").unwrap();
        git(&path, &["add", "feature.txt"]);
        git(&path, &["commit", "-m", "feature"]);

        let report = check(&fixture.pool, &fixture.locks, &task.id, local)
            .await
            .unwrap();
        assert!(report.changes.is_empty() && !report.operation_in_progress);
        assert_eq!(report.commits_ahead, 1);

        let removed = run(&fixture.pool, &fixture.locks, &task.id, local)
            .await
            .unwrap();

        assert!(removed.worktree.is_none());
        assert!(!path.exists());
        assert!(branch_exists(&fixture.repo, &worktree.branch_name));
        task_delete::run(&fixture.pool, &task.id).await.unwrap();
    }

    #[tokio::test]
    async fn blocks_removal_with_tracked_untracked_or_ignored_files_and_keeps_them() {
        let fixture = fixture().await;
        std::fs::write(fixture.repo.join(".gitignore"), ".env\n").unwrap();
        git(&fixture.repo, &["add", ".gitignore"]);
        git(&fixture.repo, &["commit", "-m", "ignore"]);
        type Dirty = fn(&Path);
        let cases: [(&str, Dirty, ChangeKind, &str); 4] = [
            (
                "tracked",
                |path| std::fs::write(path.join("README.md"), "changed").unwrap(),
                ChangeKind::Tracked,
                "README.md",
            ),
            (
                "staged",
                |path| {
                    std::fs::write(path.join("staged.txt"), "x").unwrap();
                    git(path, &["add", "staged.txt"]);
                },
                ChangeKind::Tracked,
                "staged.txt",
            ),
            (
                "untracked",
                |path| std::fs::write(path.join("notes.txt"), "x").unwrap(),
                ChangeKind::Untracked,
                "notes.txt",
            ),
            (
                "ignored",
                |path| std::fs::write(path.join(".env"), "SECRET=1").unwrap(),
                ChangeKind::Ignored,
                ".env",
            ),
        ];

        for (title, dirty, kind, file) in cases {
            let task = task(&fixture, title, "main").await;
            let path = PathBuf::from(prepare(&fixture, &task).await.worktree.unwrap().path);
            dirty(&path);

            let report = check(&fixture.pool, &fixture.locks, &task.id, local)
                .await
                .unwrap();
            assert_eq!(
                report.changes,
                [PendingChange {
                    kind,
                    path: file.to_string()
                }],
                "{title}"
            );
            assert!(matches!(
                run(&fixture.pool, &fixture.locks, &task.id, local).await,
                Err(SessionError::EnvironmentHasChanges)
            ));
            assert!(path.join(file).exists(), "{title}");
            let reloaded =
                tasks_list::one(&fixture.pool, &workspace(&fixture.pool).await, &task.id)
                    .await
                    .unwrap();
            assert_eq!(reloaded.worktree.unwrap().status, WorktreeStatus::Ready);
        }
    }

    #[tokio::test]
    async fn blocks_removal_during_a_git_operation_or_on_another_branch() {
        let fixture = fixture().await;
        let busy = task(&fixture, "Busy", "main").await;
        let busy_path = PathBuf::from(prepare(&fixture, &busy).await.worktree.unwrap().path);
        let lock = index_lock(&busy_path);
        std::fs::write(&lock, "").unwrap();
        let switched = task(&fixture, "Switched", "main").await;
        let switched_path =
            PathBuf::from(prepare(&fixture, &switched).await.worktree.unwrap().path);
        git(&switched_path, &["checkout", "-q", "-b", "elsewhere"]);

        assert!(matches!(
            run(&fixture.pool, &fixture.locks, &busy.id, local).await,
            Err(SessionError::EnvironmentOperationInProgress)
        ));
        assert!(matches!(
            run(&fixture.pool, &fixture.locks, &switched.id, local).await,
            Err(SessionError::EnvironmentIdentityMismatch)
        ));
        assert!(busy_path.exists() && switched_path.exists());
        std::fs::remove_file(lock).unwrap();
    }

    #[tokio::test]
    async fn rejects_environments_on_unsupported_storage_or_not_ready() {
        let fixture = fixture().await;
        let task = task(&fixture, "Storage", "main").await;
        prepare(&fixture, &task).await;
        let network = async |_: &Path| Err(StorageIssue::Network);

        assert!(matches!(
            run(&fixture.pool, &fixture.locks, &task.id, network).await,
            Err(SessionError::StorageNetwork)
        ));
        let other = task_without_environment(&fixture).await;
        assert!(matches!(
            run(&fixture.pool, &fixture.locks, &other, local).await,
            Err(SessionError::EnvironmentNotFound)
        ));
    }

    fn probe(marker: &Path) -> String {
        format!(
            "echo run >> '{}'; cat",
            marker.to_string_lossy().replace('\\', "/")
        )
    }

    #[tokio::test]
    async fn a_filter_that_appears_after_preparation_is_never_run_and_blocks_the_check() {
        let fixture = fixture().await;
        std::fs::write(fixture.repo.join(".gitattributes"), "*.md filter=probe\n").unwrap();
        git(&fixture.repo, &["add", ".gitattributes"]);
        git(&fixture.repo, &["commit", "-m", "attributes"]);
        let task = task(&fixture, "Late filter", "main").await;
        let path = PathBuf::from(prepare(&fixture, &task).await.worktree.unwrap().path);
        let markers = tempfile::tempdir().unwrap();
        let marker = markers.path().join("marker");
        git(
            &fixture.repo,
            &["config", "filter.probe.clean", &probe(&marker)],
        );
        std::fs::write(path.join("README.md"), "world\n").unwrap();

        let checked = check(&fixture.pool, &fixture.locks, &task.id, local).await;
        let removed = run(&fixture.pool, &fixture.locks, &task.id, local).await;

        assert!(matches!(
            checked,
            Err(SessionError::EnvironmentCheckRefused)
        ));
        assert!(matches!(
            removed,
            Err(SessionError::EnvironmentCheckRefused)
        ));
        assert!(!marker.exists());
        assert_eq!(
            std::fs::read_to_string(path.join("README.md")).unwrap(),
            "world\n"
        );
        let reloaded = tasks_list::one(&fixture.pool, &workspace(&fixture.pool).await, &task.id)
            .await
            .unwrap();
        assert_eq!(reloaded.worktree.unwrap().status, WorktreeStatus::Ready);
    }

    #[tokio::test]
    async fn attributes_written_in_the_worktree_do_not_run_a_defined_filter() {
        let fixture = fixture().await;
        let task = task(&fixture, "Worktree attributes", "main").await;
        let path = PathBuf::from(prepare(&fixture, &task).await.worktree.unwrap().path);
        let markers = tempfile::tempdir().unwrap();
        let marker = markers.path().join("marker");
        git(
            &fixture.repo,
            &["config", "filter.probe.clean", &probe(&marker)],
        );
        std::fs::write(path.join(".gitattributes"), "*.md filter=probe\n").unwrap();
        std::fs::write(path.join("README.md"), "world\n").unwrap();

        let report = check(&fixture.pool, &fixture.locks, &task.id, local)
            .await
            .unwrap();

        assert!(!marker.exists());
        assert!(
            report
                .changes
                .iter()
                .any(|change| change.path == "README.md")
        );
        assert!(
            report
                .changes
                .iter()
                .any(|change| change.path == ".gitattributes")
        );
    }

    #[tokio::test]
    async fn an_info_attributes_file_with_rules_refuses_the_check() {
        let fixture = fixture().await;
        let task = task(&fixture, "Info attributes", "main").await;
        prepare(&fixture, &task).await;
        let common = git::layout(&fixture.repo).await.unwrap().common_dir;
        let info = PathBuf::from(common).join("info");
        std::fs::create_dir_all(&info).unwrap();
        std::fs::write(info.join("attributes"), "*.md filter=probe\n").unwrap();

        let checked = check(&fixture.pool, &fixture.locks, &task.id, local).await;

        assert!(matches!(
            checked,
            Err(SessionError::EnvironmentCheckRefused)
        ));
    }

    #[tokio::test]
    async fn a_gitlink_blocks_removal_like_any_other_change() {
        let fixture = fixture().await;
        let task = task(&fixture, "Gitlink", "main").await;
        let path = PathBuf::from(prepare(&fixture, &task).await.worktree.unwrap().path);
        let nested = path.join("nested");
        std::fs::create_dir(&nested).unwrap();
        git(&nested, &["init", "-q"]);
        git(&nested, &["commit", "-q", "--allow-empty", "-m", "n"]);
        git(&path, &["add", "nested"]);

        let report = check(&fixture.pool, &fixture.locks, &task.id, local)
            .await
            .unwrap();

        assert!(report.changes.contains(&PendingChange {
            kind: ChangeKind::Tracked,
            path: "nested".into(),
        }));
        assert!(matches!(
            run(&fixture.pool, &fixture.locks, &task.id, local).await,
            Err(SessionError::EnvironmentHasChanges)
        ));
        assert!(nested.exists());
    }

    async fn task_without_environment(
        fixture: &crate::services::worktree_prepare::tests::Fixture,
    ) -> String {
        task(fixture, "Empty", "main").await.id
    }

    async fn workspace(pool: &SqlitePool) -> String {
        session_load::require(pool)
            .await
            .unwrap()
            .current_workspace_id
    }
}
