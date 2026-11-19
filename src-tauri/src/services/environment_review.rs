use crate::domain::repository::RepositoryIdentity;
use crate::domain::review::{
    ChangeCode, DiffSource, Drift, EntryKind, EntryView, EnvironmentReview, FolderListing,
    ReviewEntry, classify_drift, display_path, parse_numstat, parse_review_status, protected_path,
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
    let work_tree = Path::new(&worktree.path);
    let mut complete = listing.complete;
    let mut stored = Vec::new();
    for entry in &listing.entries {
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
