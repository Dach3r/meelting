use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum WorktreeError {
    #[error("git unavailable")]
    GitUnavailable(#[from] std::io::Error),
    #[error("git failed: {0}")]
    GitFailed(String),
}

impl Serialize for WorktreeError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub async fn create(repo: &Path, root: &Path) -> Result<PathBuf, WorktreeError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let path = root.join(nanos.to_string());
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["worktree", "add", "-b", &format!("meelting/{nanos}")])
        .arg(&path)
        .output()
        .await?;
    if output.status.success() {
        Ok(path)
    } else {
        Err(WorktreeError::GitFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn git(repo: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .status()
            .await
            .unwrap();
        assert!(status.success());
    }

    #[tokio::test]
    async fn creates_isolated_worktrees() {
        let repo = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        git(repo.path(), &["init"]).await;
        git(repo.path(), &["commit", "--allow-empty", "-m", "init"]).await;

        let first = create(repo.path(), root.path()).await.unwrap();
        let second = create(repo.path(), root.path()).await.unwrap();

        assert!(first.join(".git").exists());
        assert!(second.join(".git").exists());
        assert_ne!(first, second);
    }

    #[tokio::test]
    async fn fails_outside_a_repository() {
        let not_repo = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();

        let result = create(not_repo.path(), root.path()).await;

        assert!(matches!(result, Err(WorktreeError::GitFailed(_))));
    }
}
