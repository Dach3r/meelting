use crate::domain::review::{DiffRequest, DiffSource, FileDiff, parse_patch, protected_path};
use crate::domain::storage::StorageIssue;
use crate::services::environment_review::{guarded, kept_review, review_error, unchanged_since};
use crate::services::repository_locks::RepositoryLocks;
use crate::services::review_registry::{ReviewRegistry, file_token};
use crate::services::session_load::SessionError;
use crate::services::worktree_remove::{ready_environment, verify_identity};
use sqlx::SqlitePool;
use std::path::Path;

pub async fn run(
    pool: &SqlitePool,
    locks: &RepositoryLocks,
    registry: &ReviewRegistry,
    task_id: &str,
    request: &DiffRequest,
    check_storage: impl AsyncFn(&Path) -> Result<(), StorageIssue>,
) -> Result<FileDiff, SessionError> {
    let (_, worktree, repository) = ready_environment(pool, task_id).await?;
    let _lock = locks.acquire(&repository.git_common_dir).await;
    verify_identity(&repository, &worktree, &check_storage).await?;
    let review = kept_review(registry, task_id, &request.review_id, &worktree)?;
    let entry = review
        .entries
        .get(request.entry_id as usize)
        .filter(|entry| entry.sources.contains(&request.source))
        .ok_or(SessionError::InvalidReviewEntry)?;
    if protected_path(&entry.path) || entry.original_path.as_deref().is_some_and(protected_path) {
        return Err(SessionError::ProtectedFile);
    }
    let guard = guarded(&repository, &worktree).await?;
    unchanged_since(&guard, &review.state).await?;
    if request.source == DiffSource::Unstaged
        && file_token(Path::new(&worktree.path), &entry.path) != entry.file
    {
        return Err(SessionError::ReviewStale);
    }
    let output = guard
        .diff_file(request.source, &entry.path, entry.original_path.as_deref())
        .await
        .map_err(review_error)?;
    if output.truncated {
        return Err(SessionError::DiffTooLarge);
    }
    unchanged_since(&guard, &review.state).await?;
    guard.finish().await.map_err(review_error)?;
    Ok(parse_patch(&output.stdout, false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::review::{DriftState, LineKind};
    use crate::services::environment_review::tests::{entry, environment};

    fn lines(diff: &FileDiff) -> Vec<(LineKind, &str)> {
        diff.hunks
            .iter()
            .flat_map(|hunk| hunk.lines.iter())
            .map(|line| (line.kind, line.text.as_str()))
            .collect()
    }

    #[tokio::test]
    async fn each_source_shows_only_its_own_changes() {
        let env = environment().await;
        env.write("done.txt", "committed\n");
        env.git(&["add", "done.txt"]);
        env.git(&["commit", "-m", "done"]);
        env.write("README.md", "hello\nstaged\n");
        env.git(&["add", "README.md"]);
        env.write("README.md", "hello\nstaged\nunstaged\n");
        let review = env.review().await.unwrap();

        let staged = env
            .diff(&review, "README.md", DiffSource::Staged)
            .await
            .unwrap();
        let unstaged = env
            .diff(&review, "README.md", DiffSource::Unstaged)
            .await
            .unwrap();
        let committed = env
            .diff(&review, "done.txt", DiffSource::Branch)
            .await
            .unwrap();

        assert_eq!(
            lines(&staged),
            [(LineKind::Context, "hello"), (LineKind::Added, "staged")]
        );
        assert_eq!(
            lines(&unstaged),
            [
                (LineKind::Context, "hello"),
                (LineKind::Context, "staged"),
                (LineKind::Added, "unstaged")
            ]
        );
        assert_eq!(lines(&committed), [(LineKind::Added, "committed")]);
        assert!(staged.complete && unstaged.complete && committed.complete);
    }

    #[tokio::test]
    async fn binary_files_are_metadata_only() {
        let env = environment().await;
        std::fs::write(env.path.join("image.bin"), [0u8, 1, 2, 0, 255]).unwrap();
        env.git(&["add", "image.bin"]);
        env.git(&["commit", "-m", "binary"]);
        let review = env.review().await.unwrap();

        let diff = env
            .diff(&review, "image.bin", DiffSource::Branch)
            .await
            .unwrap();

        assert!(entry(&review, "image.bin").binary);
        assert!(diff.binary && diff.hunks.is_empty());
    }

    #[tokio::test]
    async fn protected_files_are_listed_but_their_contents_are_never_read() {
        let env = environment().await;
        env.write(".env.local", "TOKEN=hunter2\n");
        env.git(&["add", ".env.local"]);
        env.write("secrets.txt", "x\n");
        env.git(&["add", "secrets.txt"]);
        env.git(&["commit", "-m", "secrets"]);
        env.write("README.md", "edited\n");
        let review = env.review().await.unwrap();

        assert!(entry(&review, ".env.local").protected);
        for path in [".env.local", "secrets.txt"] {
            let source = entry(&review, path).sources[0];
            let outcome = env.diff(&review, path, source).await;
            assert!(
                matches!(outcome, Err(SessionError::ProtectedFile)),
                "{path}"
            );
        }
        assert!(!entry(&review, "README.md").protected);
    }

    #[tokio::test]
    async fn identifiers_from_other_tasks_unknown_or_out_of_range_are_rejected() {
        let env = environment().await;
        env.write("README.md", "edited\n");
        let review = env.review().await.unwrap();
        let id = entry(&review, "README.md").id;

        let other_task = env
            .diff_request("another-task", &review.review_id, id, DiffSource::Unstaged)
            .await;
        let unknown = env
            .diff_request(&env.task_id, "not-a-review", id, DiffSource::Unstaged)
            .await;
        let out_of_range = env
            .diff_request(&env.task_id, &review.review_id, 9_999, DiffSource::Unstaged)
            .await;
        let wrong_source = env
            .diff_request(&env.task_id, &review.review_id, id, DiffSource::Staged)
            .await;

        assert!(other_task.is_err());
        assert!(matches!(unknown, Err(SessionError::ReviewStale)));
        assert!(matches!(
            out_of_range,
            Err(SessionError::InvalidReviewEntry)
        ));
        assert!(matches!(
            wrong_source,
            Err(SessionError::InvalidReviewEntry)
        ));
    }

    #[tokio::test]
    async fn a_newer_review_invalidates_the_previous_one() {
        let env = environment().await;
        env.write("README.md", "edited\n");
        let old = env.review().await.unwrap();
        let fresh = env.review().await.unwrap();

        let stale = env.diff(&old, "README.md", DiffSource::Unstaged).await;
        let valid = env.diff(&fresh, "README.md", DiffSource::Unstaged).await;

        assert!(matches!(stale, Err(SessionError::ReviewStale)));
        assert!(valid.is_ok());
    }

    #[tokio::test]
    async fn a_change_after_the_review_makes_it_stale() {
        type Change = fn(&crate::services::environment_review::tests::Environment);
        let changes: [(&str, Change); 4] = [
            ("a new commit", |env| {
                env.write("more.txt", "m");
                env.git(&["add", "more.txt"]);
                env.git(&["commit", "-m", "more"]);
            }),
            ("another file staged", |env| {
                env.write("other.txt", "o");
                env.git(&["add", "other.txt"]);
            }),
            ("the reviewed file edited again", |env| {
                env.write("README.md", "edited again with a different size\n");
            }),
            ("the index rewritten by an external tool", |env| {
                env.git(&["add", "README.md"]);
            }),
        ];
        for (name, change) in changes {
            let env = environment().await;
            env.write("README.md", "edited\n");
            let review = env.review().await.unwrap();
            change(&env);

            let outcome = env.diff(&review, "README.md", DiffSource::Unstaged).await;

            assert!(matches!(outcome, Err(SessionError::ReviewStale)), "{name}");
            let refreshed = env.review().await.unwrap();
            let source = entry(&refreshed, "README.md").sources[0];
            assert!(
                env.diff(&refreshed, "README.md", source).await.is_ok(),
                "{name}"
            );
        }
    }

    #[tokio::test]
    async fn a_moved_base_branch_does_not_make_a_review_stale() {
        let env = environment().await;
        env.write("README.md", "edited\n");
        let review = env.review().await.unwrap();
        let repo = &env.fixture.repo;
        std::fs::write(repo.join("later.txt"), "x").unwrap();
        crate::services::repository_register::tests::git(repo, &["add", "later.txt"]);
        crate::services::repository_register::tests::git(repo, &["commit", "-m", "later"]);

        let diff = env.diff(&review, "README.md", DiffSource::Unstaged).await;

        assert!(diff.is_ok());
        assert_eq!(review.drift.state, DriftState::UpToDate);
    }

    #[tokio::test]
    async fn a_diff_beyond_the_byte_limit_is_refused_and_one_beyond_the_line_limit_is_partial() {
        let env = environment().await;
        env.write("huge.txt", &"x".repeat(1_000).repeat(700));
        env.write("long.txt", &"line\n".repeat(2_500));
        env.git(&["add", "huge.txt", "long.txt"]);
        let review = env.review().await.unwrap();

        let huge = env.diff(&review, "huge.txt", DiffSource::Staged).await;
        let long = env
            .diff(&review, "long.txt", DiffSource::Staged)
            .await
            .unwrap();

        assert!(matches!(huge, Err(SessionError::DiffTooLarge)));
        assert!(!long.complete);
        assert_eq!(lines(&long).len(), crate::domain::review::MAX_DIFF_LINES);
    }

    #[tokio::test]
    async fn a_task_without_a_review_of_its_own_cannot_read_another_tasks_diff() {
        let env = environment().await;
        env.write("README.md", "edited\n");
        let review = env.review().await.unwrap();
        let id = entry(&review, "README.md").id;
        let second =
            crate::services::worktree_prepare::tests::task(&env.fixture, "Second", "main").await;
        crate::services::worktree_prepare::tests::prepare(&env.fixture, &second).await;

        let outcome = env
            .diff_request(&second.id, &review.review_id, id, DiffSource::Unstaged)
            .await;

        assert!(matches!(outcome, Err(SessionError::ReviewStale)));
    }
}
