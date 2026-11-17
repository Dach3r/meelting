use crate::system::git::{self, GitError};
use crate::system::terminal::{self, TerminalError};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum WorktreeRunError {
    #[error(transparent)]
    Git(#[from] GitError),
    #[error(transparent)]
    Terminal(#[from] TerminalError),
}

impl Serialize for WorktreeRunError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub async fn run(
    repo: &Path,
    worktrees_root: &Path,
    command: &str,
    on_output: impl Fn(String) + Send + 'static,
    on_exit: impl FnOnce(u32) + Send + 'static,
) -> Result<PathBuf, WorktreeRunError> {
    let worktree = git::add_worktree(repo, worktrees_root).await?;
    terminal::spawn(command, &worktree, on_output, on_exit)?;
    Ok(worktree)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::sync::mpsc;
    use std::time::Duration;

    fn git(path: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .status()
            .unwrap();
        assert!(status.success());
    }

    fn init_repo(path: &Path) {
        git(path, &["init"]);
        git(path, &["commit", "--allow-empty", "-m", "init"]);
    }

    #[tokio::test]
    async fn runs_command_inside_new_worktree() {
        let repo = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        init_repo(repo.path());
        let (output_tx, output_rx) = mpsc::channel();
        let (exit_tx, exit_rx) = mpsc::channel();

        let worktree = run(
            repo.path(),
            root.path(),
            "echo ready",
            move |text| output_tx.send(text).unwrap(),
            move |code| exit_tx.send(code).unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(exit_rx.recv_timeout(Duration::from_secs(10)).unwrap(), 0);
        let output: String = output_rx.try_iter().collect();
        assert!(output.contains("ready"));
        assert!(worktree.starts_with(root.path()));
    }

    #[tokio::test]
    async fn reports_git_failure_without_spawning() {
        let not_repo = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();

        let result = run(not_repo.path(), root.path(), "echo ready", |_| {}, |_| {}).await;

        assert!(matches!(result, Err(WorktreeRunError::Git(_))));
    }
}
