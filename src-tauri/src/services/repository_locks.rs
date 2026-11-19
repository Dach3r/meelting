use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::OwnedMutexGuard;

#[derive(Default)]
pub struct RepositoryLocks(Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>);

impl RepositoryLocks {
    pub async fn acquire(&self, git_common_dir: &str) -> OwnedMutexGuard<()> {
        let lock = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entry(git_common_dir.to_string())
            .or_default()
            .clone();
        lock.lock_owned().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn serializes_the_same_repository_and_releases_on_drop() {
        let locks = RepositoryLocks::default();
        let first = locks.acquire("/repo/.git").await;

        let blocked =
            tokio::time::timeout(Duration::from_millis(50), locks.acquire("/repo/.git")).await;
        let other =
            tokio::time::timeout(Duration::from_millis(50), locks.acquire("/other/.git")).await;

        assert!(blocked.is_err());
        assert!(other.is_ok());
        drop(first);
        assert!(
            tokio::time::timeout(Duration::from_millis(50), locks.acquire("/repo/.git"))
                .await
                .is_ok()
        );
    }
}
