use crate::domain::file_copy::{
    CopyOutcome, CopyPreview, CopyProblem, CopyResult, FileToCopy, MAX_FILE_BYTES, MAX_TOTAL_BYTES,
    looks_secret, relative_path,
};
use crate::domain::storage::StorageIssue;
use crate::domain::task::Task;
use crate::domain::worktree::simplified;
use crate::repositories::task_worktree_file_copies;
use crate::services::repository_locks::RepositoryLocks;
use crate::services::session_load::SessionError;
use crate::services::tasks_list;
use crate::services::worktree_prepare::check_repository;
use crate::services::worktree_remove::ready_environment;
use crate::services::worktrees_reconcile::intact;
use crate::system::git;
use crate::system::safe_file::{self, SafeFileError};
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Default)]
pub struct CopySelections(Mutex<HashMap<String, (String, Vec<String>)>>);

impl CopySelections {
    fn keep(&self, task_id: &str, paths: Vec<String>) -> String {
        let id = Uuid::now_v7().to_string();
        let mut selections = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        selections.retain(|_, (kept_task, _)| kept_task != task_id);
        selections.insert(id.clone(), (task_id.to_string(), paths));
        id
    }

    fn take(&self, task_id: &str, id: &str) -> Option<Vec<String>> {
        let mut selections = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match selections.remove(id) {
            Some((kept_task, paths)) if kept_task == task_id => Some(paths),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CopyReport {
    pub results: Vec<CopyResult>,
    pub task: Task,
}

fn problem(error: SafeFileError) -> CopyProblem {
    match error {
        SafeFileError::SymbolicLink => CopyProblem::SymbolicLink,
        SafeFileError::Exists => CopyProblem::AlreadyInEnvironment,
        SafeFileError::NotAFile | SafeFileError::Io(_) => CopyProblem::NotAFile,
    }
}

fn relative_to(toplevel: &Path, picked: &Path) -> Result<String, CopyProblem> {
    let link = std::fs::symlink_metadata(picked).map_err(|_| CopyProblem::NotAFile)?;
    if link.file_type().is_symlink() {
        return Err(CopyProblem::SymbolicLink);
    }
    if !link.is_file() {
        return Err(CopyProblem::NotAFile);
    }
    let name = picked.file_name().ok_or(CopyProblem::NotAFile)?;
    let parent = picked.parent().ok_or(CopyProblem::OutsideRepository)?;
    let parent = std::fs::canonicalize(parent)
        .map(simplified)
        .map_err(|_| CopyProblem::OutsideRepository)?;
    let relative = parent
        .join(name)
        .strip_prefix(toplevel)
        .map(Path::to_path_buf)
        .map_err(|_| CopyProblem::OutsideRepository)?;
    relative_path(&relative)
}

async fn evaluate(
    toplevel: &Path,
    worktree: &Path,
    relative: Result<String, CopyProblem>,
) -> FileToCopy {
    let relative_path = match &relative {
        Ok(path) => path.clone(),
        Err(_) => String::new(),
    };
    let mut file = FileToCopy {
        secret: looks_secret(&relative_path),
        relative_path,
        size_bytes: 0,
        problem: relative.err(),
    };
    if file.problem.is_some() {
        return file;
    }
    match safe_file::open_source(toplevel, &file.relative_path) {
        Ok(source) => {
            file.size_bytes = source
                .metadata()
                .map(|metadata| metadata.len())
                .unwrap_or(0)
        }
        Err(error) => file.problem = Some(problem(error)),
    }
    if file.problem.is_none()
        && git::is_tracked(toplevel, &file.relative_path)
            .await
            .unwrap_or(true)
    {
        file.problem = Some(CopyProblem::Tracked);
    }
    file.problem = file.problem.or_else(|| {
        if file.size_bytes > MAX_FILE_BYTES {
            Some(CopyProblem::TooLarge)
        } else if safe_file::exists(worktree, &file.relative_path) {
            Some(CopyProblem::AlreadyInEnvironment)
        } else {
            None
        }
    });
    file
}

fn copyable(files: &[FileToCopy]) -> impl Iterator<Item = &FileToCopy> {
    files.iter().filter(|file| file.problem.is_none())
}

fn total(files: &[FileToCopy]) -> u64 {
    copyable(files).map(|file| file.size_bytes).sum()
}

pub async fn preview(
    pool: &SqlitePool,
    selections: &CopySelections,
    task_id: &str,
    pick: impl AsyncFnOnce(&Path) -> Option<Vec<PathBuf>>,
) -> Result<Option<CopyPreview>, SessionError> {
    let (_, worktree, repository) = ready_environment(pool, task_id).await?;
    let toplevel = std::fs::canonicalize(&repository.path)
        .map(simplified)
        .map_err(|_| SessionError::RepositoryUnavailable)?;
    let Some(picked) = pick(&toplevel).await else {
        return Ok(None);
    };
    let mut files: Vec<FileToCopy> = Vec::new();
    for path in picked {
        let file = evaluate(
            &toplevel,
            Path::new(&worktree.path),
            relative_to(&toplevel, &path),
        )
        .await;
        if file.relative_path.is_empty()
            || !files
                .iter()
                .any(|seen| seen.relative_path == file.relative_path)
        {
            files.push(file);
        }
    }
    let total_bytes = total(&files);
    if total_bytes > MAX_TOTAL_BYTES {
        return Err(SessionError::CopyTooLarge);
    }
    let id = selections.keep(
        task_id,
        copyable(&files)
            .map(|file| file.relative_path.clone())
            .collect(),
    );
    Ok(Some(CopyPreview {
        id,
        files,
        total_bytes,
    }))
}

pub async fn run(
    pool: &SqlitePool,
    locks: &RepositoryLocks,
    selections: &CopySelections,
    task_id: &str,
    selection_id: &str,
    accept_secrets: bool,
    check_storage: impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<CopyReport, SessionError> {
    let paths = selections
        .take(task_id, selection_id)
        .ok_or(SessionError::CopySelectionExpired)?;
    let (workspace_id, worktree, repository) = ready_environment(pool, task_id).await?;
    let _lock = locks.acquire(&repository.git_common_dir).await;
    check_repository(&repository, &check_storage).await?;
    let worktree_path = PathBuf::from(&worktree.path);
    check_storage(&worktree_path).await?;
    intact(&repository, &worktree)
        .await
        .ok_or(SessionError::EnvironmentIdentityMismatch)?;
    let toplevel = std::fs::canonicalize(&repository.path)
        .map(simplified)
        .map_err(|_| SessionError::RepositoryUnavailable)?;
    let mut files = Vec::new();
    for path in paths {
        files.push(evaluate(&toplevel, &worktree_path, Ok(path)).await);
    }
    if total(&files) > MAX_TOTAL_BYTES {
        return Err(SessionError::CopyTooLarge);
    }
    if !accept_secrets && copyable(&files).any(|file| file.secret) {
        return Err(SessionError::SecretsNotConfirmed);
    }
    let mut results = Vec::new();
    for file in files {
        let outcome = match file.problem {
            Some(problem) => CopyResult {
                relative_path: file.relative_path,
                outcome: CopyOutcome::Skipped,
                problem: Some(problem),
            },
            None => {
                copy_one(
                    pool,
                    &toplevel,
                    &worktree_path,
                    &worktree.id,
                    file.relative_path,
                )
                .await?
            }
        };
        results.push(outcome);
    }
    Ok(CopyReport {
        results,
        task: tasks_list::one(pool, &workspace_id, task_id).await?,
    })
}

async fn copy_one(
    pool: &SqlitePool,
    toplevel: &Path,
    worktree: &Path,
    worktree_id: &str,
    relative_path: String,
) -> Result<CopyResult, SessionError> {
    let failed = |relative_path: String, problem: Option<CopyProblem>| CopyResult {
        relative_path,
        outcome: if problem == Some(CopyProblem::AlreadyInEnvironment) {
            CopyOutcome::Skipped
        } else {
            CopyOutcome::Failed
        },
        problem,
    };
    let mut source = match safe_file::open_source(toplevel, &relative_path) {
        Ok(source) => source,
        Err(error) => return Ok(failed(relative_path, Some(problem(error)))),
    };
    let mode = safe_file::mode(&source).unwrap_or(0o600);
    let mut destination = match safe_file::create_destination(worktree, &relative_path, mode) {
        Ok(destination) => destination,
        Err(error) => return Ok(failed(relative_path, Some(problem(error)))),
    };
    match safe_file::copy_limited(&mut source, &mut destination, MAX_FILE_BYTES) {
        Ok(size) => {
            task_worktree_file_copies::record(pool, worktree_id, &relative_path, size).await?;
            Ok(CopyResult {
                relative_path,
                outcome: CopyOutcome::Copied,
                problem: None,
            })
        }
        Err(error) => {
            drop(destination);
            let _ = std::fs::remove_file(worktree.join(&relative_path));
            let problem =
                (error.kind() == std::io::ErrorKind::FileTooLarge).then_some(CopyProblem::TooLarge);
            Ok(failed(relative_path, problem))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::repository_register::tests::{git, local};
    use crate::services::worktree_prepare::tests::{Fixture, fixture, prepare, task};
    use crate::services::worktree_remove;

    async fn ready(fixture: &Fixture) -> (Task, PathBuf) {
        std::fs::write(fixture.repo.join(".gitignore"), ".env\nbuild/\n").unwrap();
        git(&fixture.repo, &["add", ".gitignore"]);
        git(&fixture.repo, &["commit", "-m", "ignore"]);
        let task = task(fixture, "Copies", "main").await;
        let path = PathBuf::from(prepare(fixture, &task).await.worktree.unwrap().path);
        (task, path)
    }

    async fn pick(
        fixture: &Fixture,
        selections: &CopySelections,
        task: &Task,
        files: Vec<PathBuf>,
    ) -> CopyPreview {
        super::preview(
            &fixture.pool,
            selections,
            &task.id,
            async move |_: &Path| Some(files),
        )
        .await
        .unwrap()
        .unwrap()
    }

    fn problems(preview: &CopyPreview) -> Vec<(String, Option<CopyProblem>)> {
        preview
            .files
            .iter()
            .map(|file| (file.relative_path.clone(), file.problem))
            .collect()
    }

    #[tokio::test]
    async fn copies_confirmed_ignored_and_untracked_files_without_storing_their_contents() {
        let fixture = fixture().await;
        let (task, worktree) = ready(&fixture).await;
        let selections = CopySelections::default();
        std::fs::write(fixture.repo.join(".env"), "API_KEY=secret").unwrap();
        std::fs::create_dir_all(fixture.repo.join("build/out dir")).unwrap();
        std::fs::write(fixture.repo.join("build/out dir/data ñ.json"), "{}").unwrap();
        let files = vec![
            fixture.repo.join(".env"),
            fixture.repo.join("build/out dir/data ñ.json"),
        ];

        let preview = pick(&fixture, &selections, &task, files.clone()).await;
        assert_eq!(preview.total_bytes, 16);
        assert!(preview.files[0].secret);
        assert!(matches!(
            run(
                &fixture.pool,
                &fixture.locks,
                &selections,
                &task.id,
                &preview.id,
                false,
                local
            )
            .await,
            Err(SessionError::SecretsNotConfirmed)
        ));
        assert!(!worktree.join(".env").exists());

        let preview = pick(&fixture, &selections, &task, files).await;
        let report = run(
            &fixture.pool,
            &fixture.locks,
            &selections,
            &task.id,
            &preview.id,
            true,
            local,
        )
        .await
        .unwrap();

        assert!(
            report
                .results
                .iter()
                .all(|result| result.outcome == CopyOutcome::Copied)
        );
        assert_eq!(
            std::fs::read_to_string(worktree.join(".env")).unwrap(),
            "API_KEY=secret"
        );
        assert_eq!(
            std::fs::read_to_string(worktree.join("build/out dir/data ñ.json")).unwrap(),
            "{}"
        );
        assert_eq!(
            std::fs::read_to_string(fixture.repo.join(".env")).unwrap(),
            "API_KEY=secret"
        );
        let copies = report.task.worktree.unwrap().copies;
        let names: Vec<&str> = copies
            .iter()
            .map(|copy| copy.relative_path.as_str())
            .collect();
        assert_eq!(names, [".env", "build/out dir/data ñ.json"]);
        let stored: Vec<String> =
            sqlx::query_scalar("SELECT relative_path FROM task_worktree_file_copies")
                .fetch_all(&fixture.pool)
                .await
                .unwrap();
        assert!(!stored.iter().any(|path| path.contains("API_KEY")));

        let blocked = worktree_remove::check(&fixture.pool, &fixture.locks, &task.id, local)
            .await
            .unwrap();
        assert!(blocked.changes.iter().any(|change| change.path == ".env"));
        assert!(matches!(
            worktree_remove::run(&fixture.pool, &fixture.locks, &task.id, local).await,
            Err(SessionError::EnvironmentHasChanges)
        ));
    }

    #[tokio::test]
    async fn rejects_tracked_internal_outside_and_existing_files() {
        let fixture = fixture().await;
        let (task, worktree) = ready(&fixture).await;
        let selections = CopySelections::default();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("other.txt"), "x").unwrap();
        std::fs::write(fixture.repo.join("notes.txt"), "mine").unwrap();
        std::fs::write(worktree.join("notes.txt"), "theirs").unwrap();

        let preview = pick(
            &fixture,
            &selections,
            &task,
            vec![
                fixture.repo.join("README.md"),
                fixture.repo.join(".git").join("config"),
                outside.path().join("other.txt"),
                fixture.repo.join("notes.txt"),
                fixture.repo.join("missing.txt"),
            ],
        )
        .await;

        assert_eq!(
            problems(&preview),
            [
                ("README.md".to_string(), Some(CopyProblem::Tracked)),
                (String::new(), Some(CopyProblem::GitInternal)),
                (String::new(), Some(CopyProblem::OutsideRepository)),
                (
                    "notes.txt".to_string(),
                    Some(CopyProblem::AlreadyInEnvironment)
                ),
                (String::new(), Some(CopyProblem::NotAFile)),
            ]
        );
        let report = run(
            &fixture.pool,
            &fixture.locks,
            &selections,
            &task.id,
            &preview.id,
            false,
            local,
        )
        .await
        .unwrap();
        assert!(report.results.is_empty());
        assert_eq!(
            std::fs::read_to_string(worktree.join("notes.txt")).unwrap(),
            "theirs"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn rejects_symbolic_links_and_skips_files_that_appear_before_copying() {
        let fixture = fixture().await;
        let (task, worktree) = ready(&fixture).await;
        let selections = CopySelections::default();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret"), "x").unwrap();
        std::os::unix::fs::symlink(outside.path().join("secret"), fixture.repo.join("link.txt"))
            .unwrap();
        std::fs::write(fixture.repo.join("late.txt"), "mine").unwrap();

        let preview = pick(
            &fixture,
            &selections,
            &task,
            vec![fixture.repo.join("link.txt"), fixture.repo.join("late.txt")],
        )
        .await;
        assert_eq!(preview.files[0].problem, Some(CopyProblem::SymbolicLink));
        std::fs::write(worktree.join("late.txt"), "theirs").unwrap();

        let report = run(
            &fixture.pool,
            &fixture.locks,
            &selections,
            &task.id,
            &preview.id,
            false,
            local,
        )
        .await
        .unwrap();

        assert_eq!(report.results[0].outcome, CopyOutcome::Skipped);
        assert_eq!(
            report.results[0].problem,
            Some(CopyProblem::AlreadyInEnvironment)
        );
        assert_eq!(
            std::fs::read_to_string(worktree.join("late.txt")).unwrap(),
            "theirs"
        );
        assert!(!worktree.join("link.txt").exists());
    }

    #[tokio::test]
    async fn enforces_the_size_limits() {
        let fixture = fixture().await;
        let (task, _) = ready(&fixture).await;
        let selections = CopySelections::default();
        let big = vec![0u8; (MAX_FILE_BYTES + 1) as usize];
        std::fs::write(fixture.repo.join("big.bin"), &big).unwrap();
        let mut files = Vec::new();
        for index in 0..6 {
            let path = fixture.repo.join(format!("part{index}.bin"));
            std::fs::write(&path, vec![0u8; (MAX_FILE_BYTES - 1) as usize]).unwrap();
            files.push(path);
        }

        let preview = pick(
            &fixture,
            &selections,
            &task,
            vec![fixture.repo.join("big.bin")],
        )
        .await;
        assert_eq!(preview.files[0].problem, Some(CopyProblem::TooLarge));
        assert!(matches!(
            super::preview(
                &fixture.pool,
                &selections,
                &task.id,
                async move |_: &Path| Some(files)
            )
            .await,
            Err(SessionError::CopyTooLarge)
        ));
    }

    #[tokio::test]
    async fn a_selection_is_used_once_and_only_for_its_task() {
        let fixture = fixture().await;
        let (task, _) = ready(&fixture).await;
        let other = crate::services::worktree_prepare::tests::task(&fixture, "Other", "main").await;
        let selections = CopySelections::default();
        std::fs::write(fixture.repo.join("notes.txt"), "x").unwrap();
        let preview = pick(
            &fixture,
            &selections,
            &task,
            vec![fixture.repo.join("notes.txt")],
        )
        .await;

        assert!(matches!(
            run(
                &fixture.pool,
                &fixture.locks,
                &selections,
                &other.id,
                &preview.id,
                false,
                local
            )
            .await,
            Err(SessionError::CopySelectionExpired)
        ));
        assert!(matches!(
            run(
                &fixture.pool,
                &fixture.locks,
                &selections,
                &task.id,
                &preview.id,
                false,
                local
            )
            .await,
            Err(SessionError::CopySelectionExpired)
        ));
        assert!(
            super::preview(&fixture.pool, &selections, &task.id, async |_: &Path| None)
                .await
                .unwrap()
                .is_none()
        );
    }
}
