use crate::domain::repository::RepositoryIdentity;
use crate::domain::review::{
    ChangeCode, DiffSource, Drift, EntryKind, EntryView, EnvironmentReview, FolderListing,
    ReviewEntry, classify_drift, display_path, immediate_children, parse_numstat,
    parse_review_status, protected_path,
};
use crate::domain::storage::StorageIssue;
use crate::domain::worktree::{IN_PROGRESS_MARKERS, Worktree};
use crate::services::repository_locks::RepositoryLocks;
use crate::services::review_registry::{
    REVIEW_TTL, ReviewRegistry, StoredEntry, StoredReview, file_token,
};
use crate::services::session_load::SessionError;
use crate::services::worktree_remove::{ready_environment, verify_identity};
use crate::system::git;
use crate::system::git_guarded::{Guard, GuardError, ReviewState};
use sqlx::SqlitePool;
use std::path::Path;

pub fn review_error(error: GuardError) -> SessionError {
    match error {
        GuardError::Unavailable(_) => SessionError::GitUnavailable,
        GuardError::GitTooOld => SessionError::GitTooOld,
        GuardError::IsolationUnavailable => SessionError::ProcessIsolationUnavailable,
        GuardError::FiltersApply(_) => SessionError::EnvironmentFiltersApply,
        GuardError::InfoAttributesUnsafe
        | GuardError::ConfigIncludes
        | GuardError::ConfigChanged
        | GuardError::FilterBlocked => SessionError::EnvironmentConfigUnsafe,
        GuardError::TimedOut => SessionError::ReviewTimeout,
        GuardError::OutputTooLarge => SessionError::DiffTooLarge,
        GuardError::GitDirNotFound => SessionError::EnvironmentIdentityMismatch,
        GuardError::Failed(_) => SessionError::EnvironmentCheckRefused,
    }
}

pub async fn guarded(
    repository: &RepositoryIdentity,
    worktree: &Worktree,
) -> Result<Guard, SessionError> {
    let guard = Guard::open(
        repository,
        Path::new(&worktree.path),
        &worktree.base_commit_sha,
    )
    .await
    .map_err(review_error)?;
    let expected = format!("refs/heads/{}", worktree.branch_name);
    if guard.head_reference().await.map_err(review_error)? != Some(expected) {
        return Err(SessionError::EnvironmentIdentityMismatch);
    }
    Ok(guard)
}

pub async fn unchanged_since(guard: &Guard, expected: &ReviewState) -> Result<(), SessionError> {
    if guard.review_state().await.map_err(review_error)? == *expected {
        Ok(())
    } else {
        Err(SessionError::ReviewStale)
    }
}

pub fn kept_review(
    registry: &ReviewRegistry,
    task_id: &str,
    review_id: &str,
    worktree: &Worktree,
) -> Result<StoredReview, SessionError> {
    registry
        .get(task_id, review_id, REVIEW_TTL)
        .filter(|review| review.worktree_id == worktree.id)
        .ok_or(SessionError::ReviewStale)
}

struct Counts {
    added: Option<u64>,
    deleted: Option<u64>,
    binary: bool,
}

impl Counts {
    const NONE: Counts = Counts {
        added: None,
        deleted: None,
        binary: false,
    };
}

fn stored_of(work_tree: &Path, entry: &ReviewEntry) -> StoredEntry {
    let mut sources = Vec::new();
    if entry.staged.is_some() {
        sources.push(DiffSource::Staged);
    }
    if entry.unstaged.is_some() {
        sources.push(DiffSource::Unstaged);
    }
    StoredEntry {
        kind: entry.kind,
        path: entry.path.clone(),
        original_path: entry.original_path.clone(),
        sources,
        file: file_token(work_tree, &entry.path),
    }
}

fn view_of(
    id: usize,
    stored: &StoredEntry,
    staged: Option<ChangeCode>,
    unstaged: Option<ChangeCode>,
    counts: Counts,
) -> Result<EntryView, SessionError> {
    let (path, lossy) = display_path(&stored.path);
    let original = stored.original_path.as_deref().map(display_path);
    Ok(EntryView {
        id: u32::try_from(id).map_err(|_| SessionError::ReviewStale)?,
        kind: stored.kind,
        folder: stored.path.ends_with(b"/")
            && matches!(stored.kind, EntryKind::Untracked | EntryKind::Ignored),
        protected: protected_path(&stored.path)
            || stored.original_path.as_deref().is_some_and(protected_path),
        lossy_path: lossy || original.as_ref().is_some_and(|(_, lossy)| *lossy),
        original_path: original.map(|(path, _)| path),
        path,
        staged,
        unstaged,
        sources: stored.sources.clone(),
        added: counts.added,
        deleted: counts.deleted,
        binary: counts.binary,
    })
}

async fn drift(repository: &RepositoryIdentity, worktree: &Worktree) -> Drift {
    let repository_dir = Path::new(&repository.path);
    let current = git::branch_commit(repository_dir, &worktree.base_ref)
        .await
        .ok()
        .flatten();
    let ancestor = match &current {
        Some(current) if *current != worktree.base_commit_sha => {
            git::is_ancestor(repository_dir, &worktree.base_commit_sha, current)
                .await
                .ok()
                .flatten()
        }
        _ => None,
    };
    Drift {
        state: classify_drift(&worktree.base_commit_sha, current.as_deref(), ancestor),
        base_ref: worktree.base_ref.clone(),
        current_commit: current,
    }
}

pub async fn run(
    pool: &SqlitePool,
    locks: &RepositoryLocks,
    registry: &ReviewRegistry,
    task_id: &str,
    check_storage: impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<EnvironmentReview, SessionError> {
    let (_, worktree, repository) = ready_environment(pool, task_id).await?;
    let _lock = locks.acquire(&repository.git_common_dir).await;
    verify_identity(&repository, &worktree, &check_storage).await?;
    let work_tree = Path::new(&worktree.path);
    let guard = guarded(&repository, &worktree).await?;
    let state = guard.review_state().await.map_err(review_error)?;
    let status_output = guard.review_status().await.map_err(review_error)?;
    let committed_output = guard
        .diff_numstat(DiffSource::Branch)
        .await
        .map_err(review_error)?;
    let mut operation_in_progress = false;
    for marker in IN_PROGRESS_MARKERS {
        let marker = guard.git_path(marker).await.map_err(review_error)?;
        operation_in_progress |= std::fs::symlink_metadata(marker).is_ok();
    }
    let commits_ahead = guard
        .commits_ahead(&worktree.branch_name)
        .await
        .map_err(review_error)?;
    unchanged_since(&guard, &state).await?;
    guard.finish().await.map_err(review_error)?;

    let status = parse_review_status(&status_output.stdout, status_output.truncated);
    let committed = parse_numstat(&committed_output.stdout, committed_output.truncated);
    let mut stored = Vec::new();
    let mut views = Vec::new();
    for entry in &status.entries {
        let kept = stored_of(work_tree, entry);
        views.push(view_of(
            stored.len(),
            &kept,
            entry.staged,
            entry.unstaged,
            Counts::NONE,
        )?);
        stored.push(kept);
    }
    for entry in &committed.entries {
        let kept = StoredEntry {
            kind: EntryKind::Committed,
            path: entry.path.clone(),
            original_path: entry.original_path.clone(),
            sources: vec![DiffSource::Branch],
            file: None,
        };
        let counts = Counts {
            added: entry.added,
            deleted: entry.deleted,
            binary: entry.binary(),
        };
        views.push(view_of(stored.len(), &kept, None, None, counts)?);
        stored.push(kept);
    }
    let head_commit_sha = state.head().map(str::to_string);
    let review_id = registry.keep(
        task_id,
        StoredReview {
            worktree_id: worktree.id.clone(),
            state,
            entries: stored,
        },
    );
    Ok(EnvironmentReview {
        review_id,
        branch: worktree.branch_name.clone(),
        base_commit_sha: worktree.base_commit_sha.clone(),
        head_commit_sha,
        commits_ahead,
        drift: drift(&repository, &worktree).await,
        operation_in_progress,
        entries: views,
        complete: status.complete && committed.complete,
    })
}

pub async fn folder(
    pool: &SqlitePool,
    locks: &RepositoryLocks,
    registry: &ReviewRegistry,
    task_id: &str,
    review_id: &str,
    entry_id: u32,
    check_storage: impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<FolderListing, SessionError> {
    let (_, worktree, repository) = ready_environment(pool, task_id).await?;
    let _lock = locks.acquire(&repository.git_common_dir).await;
    verify_identity(&repository, &worktree, &check_storage).await?;
    let review = kept_review(registry, task_id, review_id, &worktree)?;
    let folder = review
        .entries
        .get(entry_id as usize)
        .filter(|entry| {
            entry.path.ends_with(b"/")
                && matches!(entry.kind, EntryKind::Untracked | EntryKind::Ignored)
        })
        .ok_or(SessionError::InvalidReviewEntry)?;
    let guard = guarded(&repository, &worktree).await?;
    unchanged_since(&guard, &review.state).await?;
    let output = guard
        .review_folder(&folder.path)
        .await
        .map_err(review_error)?;
    unchanged_since(&guard, &review.state).await?;
    guard.finish().await.map_err(review_error)?;

    let listing = parse_review_status(&output.stdout, output.truncated);
    let children = immediate_children(&folder.path, listing.entries);
    let work_tree = Path::new(&worktree.path);
    let mut complete = listing.complete;
    let mut stored = Vec::new();
    for entry in &children {
        if !matches!(entry.kind, EntryKind::Untracked | EntryKind::Ignored) {
            complete = false;
            continue;
        }
        stored.push(stored_of(work_tree, entry));
    }
    let first = registry
        .append(task_id, review_id, REVIEW_TTL, stored.clone())
        .ok_or(SessionError::ReviewStale)?;
    let entries = stored
        .iter()
        .enumerate()
        .map(|(offset, kept)| view_of(first + offset, kept, None, None, Counts::NONE))
        .collect::<Result<_, _>>()?;
    Ok(FolderListing { entries, complete })
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::domain::review::{DiffRequest, DriftState};
    use crate::services::repository_register::tests::{git, local};
    use crate::services::session_load;
    use crate::services::worktree_prepare::tests::{Fixture, fixture, prepare, task};
    use crate::services::worktree_trash::tests::trashed;
    use crate::services::{environment_diff, workspace_create};
    use std::path::PathBuf;
    use std::process::Command;

    pub struct Environment {
        pub fixture: Fixture,
        pub registry: ReviewRegistry,
        pub task_id: String,
        pub path: PathBuf,
    }

    pub async fn environment() -> Environment {
        let fixture = fixture().await;
        let task = task(&fixture, "Review", "main").await;
        let path = PathBuf::from(prepare(&fixture, &task).await.worktree.unwrap().path);
        Environment {
            fixture,
            registry: ReviewRegistry::default(),
            task_id: task.id,
            path,
        }
    }

    impl Environment {
        pub async fn review(&self) -> Result<EnvironmentReview, SessionError> {
            run(
                &self.fixture.pool,
                &self.fixture.locks,
                &self.registry,
                &self.task_id,
                local,
            )
            .await
        }

        pub async fn diff(
            &self,
            review: &EnvironmentReview,
            path: &str,
            source: DiffSource,
        ) -> Result<crate::domain::review::FileDiff, SessionError> {
            self.diff_request(
                &self.task_id,
                &review.review_id,
                entry(review, path).id,
                source,
            )
            .await
        }

        pub async fn diff_request(
            &self,
            task_id: &str,
            review_id: &str,
            entry_id: u32,
            source: DiffSource,
        ) -> Result<crate::domain::review::FileDiff, SessionError> {
            let request = DiffRequest {
                review_id: review_id.to_string(),
                entry_id,
                source,
            };
            environment_diff::run(
                &self.fixture.pool,
                &self.fixture.locks,
                &self.registry,
                task_id,
                &request,
                local,
            )
            .await
        }

        pub fn write(&self, name: &str, content: &str) {
            std::fs::write(self.path.join(name), content).unwrap();
        }

        pub fn git(&self, args: &[&str]) {
            git(&self.path, args);
        }
    }

    pub fn entry<'a>(review: &'a EnvironmentReview, path: &str) -> &'a EntryView {
        review
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .unwrap_or_else(|| panic!("{path} not in {:?}", review.entries))
    }

    fn files(dir: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
        let mut found = std::collections::BTreeMap::new();
        let mut pending = vec![dir.to_path_buf()];
        while let Some(folder) = pending.pop() {
            for item in std::fs::read_dir(&folder).unwrap().flatten() {
                if item.file_type().unwrap().is_dir() {
                    pending.push(item.path());
                } else {
                    found.insert(item.path(), std::fs::read(item.path()).unwrap());
                }
            }
        }
        found
    }

    #[tokio::test]
    async fn lists_staged_unstaged_untracked_ignored_and_committed_changes_apart() {
        let env = environment().await;
        env.write(".gitignore", "build/\n");
        env.git(&["add", ".gitignore"]);
        env.git(&["commit", "-m", "ignore"]);
        env.write("committed.txt", "one\n");
        env.git(&["add", "committed.txt"]);
        env.git(&["commit", "-m", "committed"]);
        env.write("staged.txt", "s\n");
        env.git(&["add", "staged.txt"]);
        env.write("README.md", "edited\n");
        env.write("both.txt", "1\n");
        env.git(&["add", "both.txt"]);
        env.write("both.txt", "2\n");
        env.write("notes.txt", "n\n");
        std::fs::create_dir(env.path.join("build")).unwrap();
        std::fs::write(env.path.join("build/out.o"), "o").unwrap();

        let review = env.review().await.unwrap();

        assert!(review.complete && !review.operation_in_progress);
        assert_eq!(review.commits_ahead, 2);
        let kinds = |path: &str| {
            let item = entry(&review, path);
            (item.kind, item.staged.is_some(), item.unstaged.is_some())
        };
        assert_eq!(kinds("staged.txt"), (EntryKind::Changed, true, false));
        assert_eq!(kinds("README.md"), (EntryKind::Changed, false, true));
        assert_eq!(kinds("both.txt"), (EntryKind::Changed, true, true));
        assert_eq!(kinds("notes.txt"), (EntryKind::Untracked, false, false));
        assert_eq!(kinds("build/"), (EntryKind::Ignored, false, false));
        let committed = entry(&review, "committed.txt");
        assert_eq!(committed.kind, EntryKind::Committed);
        assert_eq!(committed.added, Some(1));
        assert_eq!(committed.sources, [DiffSource::Branch]);
        assert!(entry(&review, "build/").folder && !entry(&review, "notes.txt").folder);
        assert_eq!(
            entry(&review, "both.txt").sources,
            [DiffSource::Staged, DiffSource::Unstaged]
        );
    }

    #[tokio::test]
    async fn reports_the_four_drift_states() {
        let env = environment().await;
        let repo = env.fixture.repo.clone();
        let state = async || env.review().await.unwrap().drift;

        let fresh = state().await;
        assert_eq!(fresh.state, DriftState::UpToDate);

        std::fs::write(repo.join("later.txt"), "x").unwrap();
        git(&repo, &["add", "later.txt"]);
        git(&repo, &["commit", "-m", "later"]);
        let moved = state().await;
        assert_eq!(moved.state, DriftState::Moved);
        assert_ne!(moved.current_commit, fresh.current_commit);

        git(&repo, &["reset", "--hard", "-q", "HEAD~2"]);
        std::fs::write(repo.join("other.txt"), "y").unwrap();
        git(&repo, &["add", "other.txt"]);
        git(&repo, &["commit", "-m", "other"]);
        assert_eq!(state().await.state, DriftState::Rewritten);

        git(&repo, &["branch", "-m", "main", "renamed"]);
        let gone = state().await;
        assert_eq!(gone.state, DriftState::Unavailable);
        assert_eq!(gone.current_commit, None);
    }

    #[tokio::test]
    async fn drift_never_changes_the_environment_or_the_base_branch() {
        let env = environment().await;
        let repo = env.fixture.repo.clone();
        std::fs::write(repo.join("later.txt"), "x").unwrap();
        git(&repo, &["add", "later.txt"]);
        git(&repo, &["commit", "-m", "later"]);
        let before = files(&repo.join(".git"));
        let worktree_before = files(&env.path);

        env.review().await.unwrap();

        assert_eq!(files(&repo.join(".git")), before);
        assert_eq!(files(&env.path), worktree_before);
    }

    #[tokio::test]
    async fn a_merge_conflict_is_listed_and_flags_the_operation() {
        let env = environment().await;
        let repo = env.fixture.repo.clone();
        git(&repo, &["branch", "side"]);
        git(&repo, &["checkout", "-q", "side"]);
        std::fs::write(repo.join("README.md"), "side\n").unwrap();
        git(&repo, &["commit", "-qam", "side"]);
        git(&repo, &["checkout", "-q", "main"]);
        env.write("README.md", "mine\n");
        env.git(&["commit", "-qam", "mine"]);
        let merged = Command::new("git")
            .arg("-C")
            .arg(&env.path)
            .args(["-c", "user.name=t", "-c", "user.email=t@t", "merge", "side"])
            .output()
            .unwrap();
        assert!(!merged.status.success());

        let review = env.review().await.unwrap();

        assert_eq!(entry(&review, "README.md").kind, EntryKind::Conflicted);
        assert!(entry(&review, "README.md").sources.is_empty());
        assert!(review.operation_in_progress);
        let diff = env.diff(&review, "README.md", DiffSource::Unstaged).await;
        assert!(matches!(diff, Err(SessionError::InvalidReviewEntry)));
    }

    #[tokio::test]
    async fn a_rename_keeps_its_original_path() {
        let env = environment().await;
        env.git(&["mv", "README.md", "GUIDE.md"]);

        let review = env.review().await.unwrap();

        let renamed = entry(&review, "GUIDE.md");
        assert_eq!(renamed.original_path.as_deref(), Some("README.md"));
        assert_eq!(
            renamed.staged,
            Some(crate::domain::review::ChangeCode::Renamed)
        );
        let diff = env
            .diff(&review, "GUIDE.md", DiffSource::Staged)
            .await
            .unwrap();
        assert!(diff.complete && diff.hunks.is_empty() && !diff.binary);
    }

    #[tokio::test]
    async fn an_untracked_folder_is_expanded_through_the_core_only() {
        let env = environment().await;
        std::fs::create_dir_all(env.path.join("docs/deep")).unwrap();
        env.write("docs/a.md", "a");
        env.write("docs/deep/b.md", "b");
        env.write("plain.txt", "p");
        let review = env.review().await.unwrap();
        let folder = entry(&review, "docs/").id;
        let list = |entry_id: u32, review_id: String, task_id: String| {
            let env = &env;
            async move { folder_listing(env, &task_id, &review_id, entry_id).await }
        };

        let listing = list(folder, review.review_id.clone(), env.task_id.clone())
            .await
            .unwrap();

        let names: Vec<&str> = listing.entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(names, ["docs/a.md", "docs/deep/"]);
        assert!(listing.complete);
        assert!(
            listing
                .entries
                .iter()
                .all(|e| e.id as usize >= review.entries.len())
        );
        let nested = listing.entries[1].id;
        let again = list(nested, review.review_id.clone(), env.task_id.clone())
            .await
            .unwrap();
        assert_eq!(again.entries[0].path, "docs/deep/b.md");
        let file = list(
            entry(&review, "plain.txt").id,
            review.review_id.clone(),
            env.task_id.clone(),
        )
        .await;
        assert!(matches!(file, Err(SessionError::InvalidReviewEntry)));
        let unknown = list(9999, review.review_id.clone(), env.task_id.clone()).await;
        assert!(matches!(unknown, Err(SessionError::InvalidReviewEntry)));
        let other_task = list(folder, review.review_id.clone(), "other".into()).await;
        assert!(other_task.is_err());
    }

    async fn folder_listing(
        env: &Environment,
        task_id: &str,
        review_id: &str,
        entry_id: u32,
    ) -> Result<FolderListing, SessionError> {
        folder(
            &env.fixture.pool,
            &env.fixture.locks,
            &env.registry,
            task_id,
            review_id,
            entry_id,
            local,
        )
        .await
    }

    #[tokio::test]
    async fn a_very_long_status_is_cut_and_reported_as_incomplete() {
        let env = environment().await;
        std::fs::create_dir(env.path.join("many")).unwrap();
        for index in 0..2_100 {
            std::fs::write(env.path.join(format!("many/{index}.txt")), "x").unwrap();
        }
        let review = env.review().await.unwrap();
        let folder = entry(&review, "many/").id;

        let listing = folder_listing(&env, &env.task_id, &review.review_id, folder)
            .await
            .unwrap();

        assert_eq!(listing.entries.len(), crate::domain::review::MAX_ENTRIES);
        assert!(!listing.complete);
    }

    #[tokio::test]
    async fn reviews_are_isolated_by_task_and_workspace() {
        let env = environment().await;
        let first = env.review().await.unwrap();
        let second_task = task(&env.fixture, "Second", "main").await;
        let second_path = PathBuf::from(
            prepare(&env.fixture, &second_task)
                .await
                .worktree
                .unwrap()
                .path,
        );
        std::fs::write(second_path.join("only-second.txt"), "x").unwrap();
        env.write("only-first.txt", "x");

        let second = run(
            &env.fixture.pool,
            &env.fixture.locks,
            &env.registry,
            &second_task.id,
            local,
        )
        .await
        .unwrap();
        let first_again = env.review().await.unwrap();

        assert!(second.entries.iter().any(|e| e.path == "only-second.txt"));
        assert!(!second.entries.iter().any(|e| e.path == "only-first.txt"));
        assert!(
            first_again
                .entries
                .iter()
                .any(|e| e.path == "only-first.txt")
        );
        let cross = env
            .diff_request(&second_task.id, &first.review_id, 0, DiffSource::Staged)
            .await;
        assert!(matches!(cross, Err(SessionError::ReviewStale)));

        workspace_create::run(&env.fixture.pool, "Other")
            .await
            .unwrap();
        let hidden = env.review().await;
        assert!(matches!(hidden, Err(SessionError::TaskNotFound)));
        assert!(session_load::require(&env.fixture.pool).await.is_ok());
    }

    #[tokio::test]
    async fn needs_a_ready_environment_that_is_not_in_the_trash() {
        let env = environment().await;
        let without = task(&env.fixture, "No environment", "main").await;
        let missing = run(
            &env.fixture.pool,
            &env.fixture.locks,
            &env.registry,
            &without.id,
            local,
        )
        .await;
        assert!(matches!(missing, Err(SessionError::EnvironmentNotFound)));

        let (trashed_id, _) = trashed(&env.fixture, "In the trash").await;
        let in_trash = run(
            &env.fixture.pool,
            &env.fixture.locks,
            &env.registry,
            &trashed_id,
            local,
        )
        .await;
        assert!(matches!(in_trash, Err(SessionError::EnvironmentInTrash)));
    }

    #[tokio::test]
    async fn an_executable_filter_used_by_the_base_commit_refuses_the_review() {
        let fixture = fixture().await;
        std::fs::write(
            fixture.repo.join(".gitattributes"),
            "*.md filter=meeltingprobe\n",
        )
        .unwrap();
        git(&fixture.repo, &["add", ".gitattributes"]);
        git(&fixture.repo, &["commit", "-m", "attributes"]);
        let task = task(&fixture, "Filtered", "main").await;
        prepare(&fixture, &task).await;
        git(
            &fixture.repo,
            &["config", "filter.meeltingprobe.clean", "cat"],
        );
        let registry = ReviewRegistry::default();

        let outcome = run(&fixture.pool, &fixture.locks, &registry, &task.id, local).await;

        assert!(matches!(
            outcome,
            Err(SessionError::EnvironmentFiltersApply)
        ));
    }

    #[tokio::test]
    async fn included_configuration_is_refused_with_a_typed_error() {
        let env = environment().await;
        let config = env.fixture.repo.join(".git/config");
        let original = std::fs::read_to_string(&config).unwrap();
        std::fs::write(
            &config,
            format!("{original}[include]\n\tpath = elsewhere.cfg\n"),
        )
        .unwrap();

        let outcome = env.review().await;

        assert!(matches!(
            outcome,
            Err(SessionError::EnvironmentConfigUnsafe)
        ));
    }

    #[test]
    fn guard_failures_map_to_typed_session_errors() {
        use GuardError::*;
        let cases: [(GuardError, &str); 9] = [
            (GitTooOld, "git_too_old"),
            (IsolationUnavailable, "process_isolation_unavailable"),
            (FiltersApply(vec![]), "environment_filters_apply"),
            (ConfigIncludes, "environment_config_unsafe"),
            (FilterBlocked, "environment_config_unsafe"),
            (TimedOut, "review_timeout"),
            (OutputTooLarge, "diff_too_large"),
            (GitDirNotFound, "environment_identity_mismatch"),
            (Failed(None), "environment_check_refused"),
        ];
        for (error, code) in cases {
            assert_eq!(review_error(error).to_string(), code);
        }
    }

    #[tokio::test]
    async fn reviewing_never_writes_to_the_repository_or_the_environment() {
        let env = environment().await;
        env.write("staged.txt", "s");
        env.git(&["add", "staged.txt"]);
        env.write("README.md", "edited\n");
        env.write("new.txt", "n");
        let common = env.fixture.repo.join(".git");
        let before = files(&common);
        let tree_before = files(&env.path);

        let review = env.review().await.unwrap();
        env.diff(&review, "staged.txt", DiffSource::Staged)
            .await
            .unwrap();
        env.diff(&review, "README.md", DiffSource::Unstaged)
            .await
            .unwrap();

        assert_eq!(files(&common), before);
        assert_eq!(files(&env.path), tree_before);
    }

    #[tokio::test]
    async fn concurrent_reviews_serialize_and_only_the_last_stays_valid() {
        let env = environment().await;
        env.write("README.md", "edited\n");

        let (first, second) = tokio::join!(env.review(), env.review());
        let (first, second) = (first.unwrap(), second.unwrap());

        let valid = |review: &EnvironmentReview| {
            let id = entry(review, "README.md").id;
            (review.review_id.clone(), id)
        };
        let mut live = 0;
        for review in [&first, &second] {
            let (id, entry_id) = valid(review);
            let outcome = env
                .diff_request(&env.task_id, &id, entry_id, DiffSource::Unstaged)
                .await;
            live += usize::from(outcome.is_ok());
        }
        assert_eq!(live, 1);
    }
}
