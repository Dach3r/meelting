use std::path::Path;
use tokio::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("git unavailable")]
    Unavailable(#[from] std::io::Error),
    #[error("git failed: {0}")]
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub toplevel: String,
    pub git_dir: String,
    pub common_dir: String,
}

const NO_HOOKS: &str = if cfg!(windows) {
    "core.hooksPath=NUL"
} else {
    "core.hooksPath=/dev/null"
};

fn git(dir: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(dir)
        .args(["-c", NO_HOOKS, "-c", "core.fsmonitor=false"])
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_CONFIG_PARAMETERS")
        .env_remove("GIT_CONFIG_COUNT")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0");
    #[cfg(windows)]
    command
        .args(["-c", "core.longpaths=true"])
        .creation_flags(0x0800_0000);
    command
}

async fn output(dir: &Path, args: &[&str], input: Option<Vec<u8>>) -> Result<Vec<u8>, GitError> {
    use tokio::io::AsyncWriteExt;
    let mut command = git(dir);
    command
        .args(args)
        .stdin(if input.is_some() {
            std::process::Stdio::piped()
        } else {
            std::process::Stdio::null()
        })
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn()?;
    if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
        tokio::spawn(async move {
            let _ = stdin.write_all(&input).await;
        });
    }
    let output = child.wait_with_output().await?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(GitError::Failed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

async fn read(dir: &Path, args: &[&str]) -> Result<String, GitError> {
    let output = git(dir).args(args).output().await?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_string())
    } else {
        Err(GitError::Failed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

async fn succeeds(dir: &Path, args: &[&str]) -> Result<bool, GitError> {
    Ok(git(dir).args(args).output().await?.status.success())
}

pub async fn is_bare(dir: &Path) -> Result<bool, GitError> {
    Ok(read(dir, &["rev-parse", "--is-bare-repository"]).await? == "true")
}

pub async fn layout(dir: &Path) -> Result<Layout, GitError> {
    let output = read(
        dir,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--show-toplevel",
            "--git-dir",
            "--git-common-dir",
        ],
    )
    .await?;
    match output.lines().collect::<Vec<_>>()[..] {
        [toplevel, git_dir, common_dir] => Ok(Layout {
            toplevel: toplevel.to_string(),
            git_dir: git_dir.to_string(),
            common_dir: common_dir.to_string(),
        }),
        _ => Err(GitError::Failed(output)),
    }
}

pub async fn has_commits(dir: &Path) -> Result<bool, GitError> {
    succeeds(dir, &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"]).await
}

pub async fn has_submodules(dir: &Path, revision: &str) -> Result<bool, GitError> {
    succeeds(dir, &["cat-file", "-e", &format!("{revision}:.gitmodules")]).await
}

pub async fn version(dir: &Path) -> Result<String, GitError> {
    read(dir, &["version"]).await
}

pub async fn branch_commit(dir: &Path, branch: &str) -> Result<Option<String>, GitError> {
    let reference = format!("refs/heads/{branch}^{{commit}}");
    let output = git(dir)
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            &reference,
        ])
        .output()
        .await?;
    Ok(output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string()))
}

pub async fn branch_exists(dir: &Path, branch: &str) -> Result<bool, GitError> {
    succeeds(
        dir,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ],
    )
    .await
}

pub async fn filters_at(dir: &Path, commit: &str) -> Result<Vec<String>, GitError> {
    let paths = output(dir, &["ls-tree", "-r", "-z", "--name-only", commit], None).await?;
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let attributes = output(
        dir,
        &["check-attr", "--source", commit, "-z", "--stdin", "filter"],
        Some(paths),
    )
    .await?;
    let fields: Vec<&[u8]> = attributes.split(|byte| *byte == 0).collect();
    let mut names: Vec<String> = fields
        .chunks(3)
        .filter_map(|triple| triple.get(2))
        .map(|value| String::from_utf8_lossy(value).into_owned())
        .filter(|value| !["unspecified", "unset", "set", ""].contains(&value.as_str()))
        .collect();
    names.sort();
    names.dedup();
    Ok(names)
}

pub async fn filter_is_executable(dir: &Path, name: &str) -> Result<bool, GitError> {
    for kind in ["clean", "smudge", "process"] {
        let output = git(dir)
            .args(["config", "--get", &format!("filter.{name}.{kind}")])
            .output()
            .await?;
        if output.status.success() && !output.stdout.trim_ascii().is_empty() {
            return Ok(true);
        }
    }
    Ok(false)
}

pub async fn add_worktree(
    dir: &Path,
    path: &Path,
    branch: &str,
    commit: &str,
) -> Result<(), GitError> {
    let path = path
        .to_str()
        .ok_or_else(|| GitError::Failed("path".into()))?;
    output(
        dir,
        &["worktree", "add", "--no-track", "-b", branch, path, commit],
        None,
    )
    .await
    .map(|_| ())
}

pub async fn worktree_list(dir: &Path) -> Result<String, GitError> {
    let output = output(dir, &["worktree", "list", "--porcelain", "-z"], None).await?;
    Ok(String::from_utf8_lossy(&output).into_owned())
}

pub async fn head(dir: &Path) -> Result<String, GitError> {
    read(dir, &["rev-parse", "--verify", "HEAD^{commit}"]).await
}

pub async fn local_branches(dir: &Path) -> Result<Vec<String>, GitError> {
    let output = read(dir, &["for-each-ref", "--format=%(refname)", "refs/heads"]).await?;
    Ok(output
        .lines()
        .filter_map(|line| line.strip_prefix("refs/heads/"))
        .map(String::from)
        .collect())
}

pub async fn symbolic_ref(dir: &Path, name: &str) -> Result<Option<String>, GitError> {
    let output = git(dir)
        .args(["symbolic-ref", "--quiet", name])
        .output()
        .await?;
    Ok(output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string()))
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

    async fn repo_with_commit() -> tempfile::TempDir {
        let repo = tempfile::tempdir().unwrap();
        git(repo.path(), &["init", "--initial-branch=main"]).await;
        git(repo.path(), &["commit", "--allow-empty", "-m", "init"]).await;
        repo
    }

    #[tokio::test]
    async fn reads_the_layout_of_the_main_working_tree_from_a_subfolder() {
        let repo = repo_with_commit().await;
        let nested = repo.path().join("src con espacios ñ");
        std::fs::create_dir(&nested).unwrap();

        let layout = layout(&nested).await.unwrap();

        let toplevel = std::fs::canonicalize(&layout.toplevel).unwrap();
        assert_eq!(toplevel, std::fs::canonicalize(repo.path()).unwrap());
        assert_eq!(layout.git_dir, layout.common_dir);
        assert!(Path::new(&layout.common_dir).is_absolute());
        assert!(!is_bare(&nested).await.unwrap());
    }

    #[tokio::test]
    async fn a_linked_worktree_shares_the_common_dir_but_not_the_git_dir() {
        let repo = repo_with_commit().await;
        let root = tempfile::tempdir().unwrap();
        let worktree = root.path().join("linked");
        git(
            repo.path(),
            &[
                "worktree",
                "add",
                "-b",
                "linked",
                worktree.to_str().unwrap(),
            ],
        )
        .await;

        let main = layout(repo.path()).await.unwrap();
        let linked = layout(&worktree).await.unwrap();

        assert_eq!(linked.common_dir, main.common_dir);
        assert_ne!(linked.git_dir, linked.common_dir);
    }

    #[tokio::test]
    async fn detects_bare_repositories() {
        let repo = tempfile::tempdir().unwrap();
        git(repo.path(), &["init", "--bare"]).await;

        assert!(is_bare(repo.path()).await.unwrap());
    }

    #[tokio::test]
    async fn detects_commits_and_submodules() {
        let empty = tempfile::tempdir().unwrap();
        git(empty.path(), &["init"]).await;
        let repo = repo_with_commit().await;

        assert!(!has_commits(empty.path()).await.unwrap());
        assert!(has_commits(repo.path()).await.unwrap());
        assert!(!has_submodules(repo.path(), "HEAD").await.unwrap());

        std::fs::write(repo.path().join(".gitmodules"), "").unwrap();
        git(repo.path(), &["add", ".gitmodules"]).await;
        git(repo.path(), &["commit", "-m", "modules"]).await;

        assert!(has_submodules(repo.path(), "HEAD").await.unwrap());
    }

    #[tokio::test]
    async fn lists_local_branches_and_symbolic_refs() {
        let repo = repo_with_commit().await;
        git(repo.path(), &["branch", "feature/login"]).await;
        git(repo.path(), &["tag", "main"]).await;

        let branches = local_branches(repo.path()).await.unwrap();

        assert_eq!(branches, ["feature/login", "main"]);
        assert_eq!(
            symbolic_ref(repo.path(), "HEAD").await.unwrap().as_deref(),
            Some("refs/heads/main")
        );
        assert_eq!(
            symbolic_ref(repo.path(), "refs/remotes/origin/HEAD")
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn fails_outside_a_repository() {
        let not_repo = tempfile::tempdir().unwrap();

        assert!(matches!(
            layout(not_repo.path()).await,
            Err(GitError::Failed(_))
        ));
    }
}
