use crate::domain::review::{DiffSource, EntryKind};
use crate::system::git_guarded::ReviewState;
use std::collections::HashMap;
use std::path::{Component, Path};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};
use uuid::Uuid;

pub const REVIEW_TTL: Duration = Duration::from_secs(15 * 60);
const MAX_STORED_ENTRIES: usize = 10_000;

pub type FileToken = (u64, SystemTime);

#[derive(Debug, Clone)]
pub struct StoredEntry {
    pub kind: EntryKind,
    pub path: Vec<u8>,
    pub original_path: Option<Vec<u8>>,
    pub sources: Vec<DiffSource>,
    pub file: Option<FileToken>,
}

#[derive(Debug, Clone)]
pub struct StoredReview {
    pub worktree_id: String,
    pub state: ReviewState,
    pub entries: Vec<StoredEntry>,
}

struct Kept {
    task_id: String,
    created: Instant,
    review: StoredReview,
}

#[derive(Default)]
pub struct ReviewRegistry(Mutex<HashMap<String, Kept>>);

impl ReviewRegistry {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Kept>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn keep(&self, task_id: &str, review: StoredReview) -> String {
        let id = Uuid::now_v7().to_string();
        let mut kept = self.lock();
        kept.retain(|_, other| other.task_id != task_id && other.created.elapsed() < REVIEW_TTL);
        kept.insert(
            id.clone(),
            Kept {
                task_id: task_id.to_string(),
                created: Instant::now(),
                review,
            },
        );
        id
    }

    pub fn get(&self, task_id: &str, id: &str, ttl: Duration) -> Option<StoredReview> {
        self.lock()
            .get(id)
            .filter(|kept| kept.task_id == task_id && kept.created.elapsed() < ttl)
            .map(|kept| kept.review.clone())
    }

    pub fn append(
        &self,
        task_id: &str,
        id: &str,
        ttl: Duration,
        entries: Vec<StoredEntry>,
    ) -> Option<usize> {
        let mut kept = self.lock();
        let kept = kept
            .get_mut(id)
            .filter(|kept| kept.task_id == task_id && kept.created.elapsed() < ttl)?;
        let first = kept.review.entries.len();
        if first + entries.len() > MAX_STORED_ENTRIES {
            return None;
        }
        kept.review.entries.extend(entries);
        Some(first)
    }
}

pub fn file_token(work_tree: &Path, path: &[u8]) -> Option<FileToken> {
    #[cfg(unix)]
    let relative = {
        use std::os::unix::ffi::OsStrExt;
        Path::new(std::ffi::OsStr::from_bytes(path))
    };
    #[cfg(not(unix))]
    let relative = Path::new(std::str::from_utf8(path).ok()?);
    if !relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        return None;
    }
    let metadata = std::fs::symlink_metadata(work_tree.join(relative)).ok()?;
    Some((metadata.len(), metadata.modified().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn review() -> StoredReview {
        StoredReview {
            worktree_id: "w".into(),
            state: ReviewState::default(),
            entries: Vec::new(),
        }
    }

    #[test]
    fn a_review_is_only_valid_for_its_own_task_and_before_it_expires() {
        let registry = ReviewRegistry::default();
        let id = registry.keep("task-a", review());

        assert!(registry.get("task-a", &id, REVIEW_TTL).is_some());
        assert!(registry.get("task-b", &id, REVIEW_TTL).is_none());
        assert!(registry.get("task-a", "unknown", REVIEW_TTL).is_none());
        assert!(registry.get("task-a", &id, Duration::ZERO).is_none());
    }

    #[test]
    fn a_newer_review_of_the_same_task_replaces_the_older_one() {
        let registry = ReviewRegistry::default();
        let old = registry.keep("task-a", review());
        let other = registry.keep("task-b", review());
        let new = registry.keep("task-a", review());

        assert!(registry.get("task-a", &old, REVIEW_TTL).is_none());
        assert!(registry.get("task-a", &new, REVIEW_TTL).is_some());
        assert!(registry.get("task-b", &other, REVIEW_TTL).is_some());
    }

    #[test]
    fn appended_entries_are_bounded_and_need_a_valid_review() {
        let registry = ReviewRegistry::default();
        let id = registry.keep("task-a", review());
        let entry = StoredEntry {
            kind: EntryKind::Untracked,
            path: b"a".to_vec(),
            original_path: None,
            sources: Vec::new(),
            file: None,
        };

        assert_eq!(
            registry.append("task-a", &id, REVIEW_TTL, vec![entry.clone()]),
            Some(0)
        );
        assert_eq!(
            registry.append("task-a", &id, REVIEW_TTL, vec![entry.clone()]),
            Some(1)
        );
        assert_eq!(
            registry.append("task-b", &id, REVIEW_TTL, vec![entry.clone()]),
            None
        );
        assert_eq!(
            registry.append("task-a", &id, REVIEW_TTL, vec![entry; MAX_STORED_ENTRIES]),
            None
        );
    }

    #[test]
    fn file_tokens_ignore_paths_that_leave_the_worktree() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "x").unwrap();

        assert!(file_token(dir.path(), b"a.txt").is_some());
        assert!(file_token(dir.path(), b"missing.txt").is_none());
        assert!(file_token(dir.path(), b"../a.txt").is_none());
        assert!(file_token(dir.path(), b"/etc/passwd").is_none());
    }
}
