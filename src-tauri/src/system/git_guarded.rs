use crate::domain::repository::RepositoryIdentity;
use crate::domain::worktree::{
    git_supports_attribute_source, info_attributes_allowed, parse_executable_filters,
    parse_gitlinks,
};
use crate::system::git::{self, GitError};
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::time::{Instant, timeout_at};

#[derive(Debug, thiserror::Error)]
pub enum GuardError {
    #[error("git unavailable")]
    Unavailable(#[from] std::io::Error),
    #[error("git too old")]
    GitTooOld,
    #[error("git directory of the worktree not found")]
    GitDirNotFound,
    #[error("info/attributes is not allowed")]
    InfoAttributesUnsafe,
    #[error("an executable filter applies to the repository")]
    FiltersApply(Vec<String>),
    #[error("the git configuration changed during the query")]
    ConfigChanged,
    #[error("git timed out")]
    TimedOut,
    #[error("git output too large")]
    OutputTooLarge,
    #[error("git failed")]
    Failed(Option<i32>),
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub timeout: Duration,
    pub max_stdout: usize,
}

pub const REMOVAL_LIMITS: Limits = Limits {
    timeout: Duration::from_secs(60),
    max_stdout: 32 * 1024 * 1024,
};

const SETTINGS_MAX_BYTES: u64 = 1024 * 1024;

const NULL_DEVICE: &str = if cfg!(windows) { "NUL" } else { "/dev/null" };

#[derive(Debug)]
pub struct Output {
    pub stdout: Vec<u8>,
    pub truncated: bool,
}

pub fn inherited_git_variables(names: impl Iterator<Item = OsString>) -> Vec<OsString> {
    names
        .filter(|name| {
            name.to_string_lossy()
                .to_ascii_uppercase()
                .starts_with("GIT_")
        })
        .collect()
}

fn command(isolated: bool) -> Command {
    let mut command = Command::new("git");
    for name in inherited_git_variables(std::env::vars_os().map(|(name, _)| name)) {
        command.env_remove(name);
    }
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0");
    if isolated {
        command
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", NULL_DEVICE)
            .env("GIT_ATTR_NOSYSTEM", "1");
    }
    #[cfg(windows)]
    command
        .args(["-c", "core.longpaths=true"])
        .creation_flags(0x0800_0000);
    command
}

async fn terminate(child: &mut Child) {
    #[cfg(unix)]
    if let Some(pid) = child.id() {
        unsafe {
            libc::killpg(pid as libc::pid_t, libc::SIGKILL);
        }
    }
    let _ = child.kill().await;
}

async fn execute(mut command: Command, limits: Limits) -> Result<Output, GuardError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn()?;
    let (Some(mut stdout), Some(mut stderr)) = (child.stdout.take(), child.stderr.take()) else {
        terminate(&mut child).await;
        return Err(GuardError::Failed(None));
    };
    tokio::spawn(async move {
        let _ = tokio::io::copy(&mut stderr, &mut tokio::io::sink()).await;
    });
    let deadline = Instant::now() + limits.timeout;
    let mut buffer = Vec::new();
    let read = timeout_at(
        deadline,
        (&mut stdout)
            .take(limits.max_stdout as u64 + 1)
            .read_to_end(&mut buffer),
    )
    .await;
    let truncated = buffer.len() > limits.max_stdout;
    let failure = match read {
        Err(_) => Some(GuardError::TimedOut),
        Ok(Err(error)) => Some(GuardError::Unavailable(error)),
        Ok(Ok(_)) => None,
    };
    if failure.is_some() || truncated {
        terminate(&mut child).await;
        if let Some(error) = failure {
            return Err(error);
        }
        buffer.truncate(limits.max_stdout);
        return Ok(Output {
            stdout: buffer,
            truncated: true,
        });
    }
    match timeout_at(deadline, child.wait()).await {
        Err(_) => {
            terminate(&mut child).await;
            Err(GuardError::TimedOut)
        }
        Ok(Err(error)) => Err(GuardError::Unavailable(error)),
        Ok(Ok(status)) if status.success() => Ok(Output {
            stdout: buffer,
            truncated: false,
        }),
        Ok(Ok(status)) => Err(GuardError::Failed(status.code())),
    }
}

fn flag(name: &str, path: &Path) -> OsString {
    let mut argument = OsString::from(name);
    argument.push(path);
    argument
}

fn from_git_error(error: GitError) -> GuardError {
    match error {
        GitError::Unavailable(error) => GuardError::Unavailable(error),
        GitError::Failed(_) => GuardError::Failed(None),
    }
}

fn read_bounded(path: &Path) -> Result<Option<Vec<u8>>, GuardError> {
    use std::io::Read;
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(GuardError::Unavailable(error)),
    };
    let mut content = Vec::new();
    file.take(SETTINGS_MAX_BYTES + 1)
        .read_to_end(&mut content)?;
    if content.len() as u64 > SETTINGS_MAX_BYTES {
        return Err(GuardError::Failed(None));
    }
    Ok(Some(content))
}

fn read_info_attributes(common_dir: &Path) -> Result<Option<Vec<u8>>, GuardError> {
    let path = common_dir.join("info").join("attributes");
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => return Err(GuardError::InfoAttributesUnsafe),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(GuardError::InfoAttributesUnsafe),
    }
    let content = read_bounded(&path)
        .map_err(|_| GuardError::InfoAttributesUnsafe)?
        .ok_or(GuardError::InfoAttributesUnsafe)?;
    if info_attributes_allowed(&content) {
        Ok(Some(content))
    } else {
        Err(GuardError::InfoAttributesUnsafe)
    }
}

pub fn admin_dir(common_dir: &Path, work_tree: &Path) -> Result<PathBuf, GuardError> {
    let work_tree = std::fs::canonicalize(work_tree).map_err(|_| GuardError::GitDirNotFound)?;
    let entries =
        std::fs::read_dir(common_dir.join("worktrees")).map_err(|_| GuardError::GitDirNotFound)?;
    let mut found = None;
    for entry in entries.flatten() {
        let is_real_folder = entry
            .file_type()
            .is_ok_and(|kind| kind.is_dir() && !kind.is_symlink());
        let Some(pointer) = is_real_folder
            .then(|| read_bounded(&entry.path().join("gitdir")).ok().flatten())
            .flatten()
        else {
            continue;
        };
        let pointer = PathBuf::from(String::from_utf8_lossy(&pointer).trim());
        let points_here = pointer.file_name() == Some(OsStr::new(".git"))
            && pointer
                .parent()
                .and_then(|parent| std::fs::canonicalize(parent).ok())
                .is_some_and(|parent| parent == work_tree);
        if points_here {
            if found.is_some() {
                return Err(GuardError::GitDirNotFound);
            }
            found = Some(entry.path());
        }
    }
    found.ok_or(GuardError::GitDirNotFound)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Fingerprint {
    common_config: Option<Vec<u8>>,
    worktree_config: Option<Vec<u8>>,
    info_attributes: Option<Vec<u8>>,
}

fn fingerprint(common_dir: &Path, admin_dir: &Path) -> Result<Fingerprint, GuardError> {
    Ok(Fingerprint {
        common_config: read_bounded(&common_dir.join("config"))?,
        worktree_config: read_bounded(&admin_dir.join("config.worktree"))?,
        info_attributes: read_info_attributes(common_dir)?,
    })
}

#[derive(Debug)]
pub struct Guard {
    admin_dir: PathBuf,
    work_tree: PathBuf,
    common_dir: PathBuf,
    base_commit: String,
    conversions: Vec<(&'static str, String)>,
    filters: BTreeSet<String>,
    fingerprint: Fingerprint,
}

impl Guard {
    pub async fn open(
        repository: &RepositoryIdentity,
        work_tree: &Path,
        base_commit: &str,
    ) -> Result<Guard, GuardError> {
        let mut version = command(false);
        version.arg("version");
        let version = execute(version, REMOVAL_LIMITS).await?;
        if !git_supports_attribute_source(&String::from_utf8_lossy(&version.stdout)) {
            return Err(GuardError::GitTooOld);
        }
        let hexadecimal = base_commit.bytes().all(|byte| byte.is_ascii_hexdigit());
        if !hexadecimal || ![40, 64].contains(&base_commit.len()) {
            return Err(GuardError::Failed(None));
        }
        let common_dir = PathBuf::from(&repository.git_common_dir);
        let admin_dir = admin_dir(&common_dir, work_tree)?;
        let fingerprint = fingerprint(&common_dir, &admin_dir)?;
        let mut guard = Guard {
            admin_dir,
            work_tree: work_tree.to_path_buf(),
            common_dir,
            base_commit: base_commit.to_string(),
            conversions: Vec::new(),
            filters: BTreeSet::new(),
            fingerprint,
        };
        guard.conversions = guard.effective_conversions().await?;
        guard.filters = guard.executable_filters().await?;
        if !guard.filters.is_empty() {
            let used = git::filters_at(Path::new(&repository.path), base_commit)
                .await
                .map_err(from_git_error)?;
            let applying: Vec<String> = used
                .into_iter()
                .filter(|name| guard.filters.contains(name))
                .collect();
            if !applying.is_empty() {
                return Err(GuardError::FiltersApply(applying));
            }
        }
        Ok(guard)
    }

    fn located(&self, isolated: bool) -> Command {
        let mut command = command(isolated);
        command
            .current_dir(&self.work_tree)
            .arg(flag("--git-dir=", &self.admin_dir))
            .arg(flag("--work-tree=", &self.work_tree));
        command
    }

    async fn effective_conversions(&self) -> Result<Vec<(&'static str, String)>, GuardError> {
        let mut conversions = Vec::new();
        for (key, allowed) in [
            ("core.autocrlf", &["true", "false", "input"][..]),
            ("core.eol", &["lf", "crlf", "native"][..]),
        ] {
            let mut read = self.located(false);
            read.args(["config", "--get", key]);
            match execute(read, REMOVAL_LIMITS).await {
                Ok(output) => {
                    let value = String::from_utf8_lossy(&output.stdout)
                        .trim()
                        .to_ascii_lowercase();
                    if allowed.contains(&value.as_str()) {
                        conversions.push((key, value));
                    }
                }
                Err(GuardError::Failed(Some(1))) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(conversions)
    }

    async fn executable_filters(&self) -> Result<BTreeSet<String>, GuardError> {
        let mut read = self.located(true);
        read.args([
            "config",
            "--null",
            "--get-regexp",
            r"^filter\..*\.(clean|smudge|process)$",
        ]);
        match execute(read, REMOVAL_LIMITS).await {
            Ok(output) => Ok(parse_executable_filters(&output.stdout)),
            Err(GuardError::Failed(Some(1))) => Ok(BTreeSet::new()),
            Err(error) => Err(error),
        }
    }

    fn unchanged(&self) -> Result<(), GuardError> {
        if fingerprint(&self.common_dir, &self.admin_dir)? == self.fingerprint {
            Ok(())
        } else {
            Err(GuardError::ConfigChanged)
        }
    }

    fn hardened(&self) -> Command {
        let mut command = self.located(true);
        command
            .arg(flag("--attr-source=", Path::new(&self.base_commit)))
            .args(["-c", "core.fsmonitor=false"])
            .args(["-c", "diff.autoRefreshIndex=false"])
            .arg("-c")
            .arg(format!("core.hooksPath={NULL_DEVICE}"))
            .arg("-c")
            .arg(format!("core.attributesFile={NULL_DEVICE}"));
        for (key, value) in &self.conversions {
            command.arg("-c").arg(format!("{key}={value}"));
        }
        command
    }

    pub async fn query_partial<I, S>(&self, args: I, limits: Limits) -> Result<Output, GuardError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = self.hardened();
        command.args(args);
        let output = execute(command, limits).await?;
        self.unchanged()?;
        Ok(output)
    }

    pub async fn query<I, S>(&self, args: I, limits: Limits) -> Result<Vec<u8>, GuardError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = self.query_partial(args, limits).await?;
        if output.truncated {
            return Err(GuardError::OutputTooLarge);
        }
        Ok(output.stdout)
    }

    pub async fn finish(self) -> Result<(), GuardError> {
        self.unchanged()?;
        if self.executable_filters().await? != self.filters {
            return Err(GuardError::ConfigChanged);
        }
        Ok(())
    }

    pub async fn head_reference(&self) -> Result<Option<String>, GuardError> {
        match self
            .query(["symbolic-ref", "--quiet", "HEAD"], REMOVAL_LIMITS)
            .await
        {
            Ok(output) => Ok(Some(String::from_utf8_lossy(&output).trim().to_string())),
            Err(GuardError::Failed(Some(1))) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub async fn status_exhaustive(&self) -> Result<Vec<u8>, GuardError> {
        self.query(
            [
                "status",
                "--porcelain=v2",
                "-z",
                "--untracked-files=all",
                "--ignored=matching",
                "--ignore-submodules=all",
            ],
            REMOVAL_LIMITS,
        )
        .await
    }

    pub async fn gitlinks(&self) -> Result<Vec<String>, GuardError> {
        let listing = self
            .query(["ls-files", "--stage", "-z"], REMOVAL_LIMITS)
            .await?;
        Ok(parse_gitlinks(&listing))
    }

    pub async fn git_path(&self, name: &str) -> Result<PathBuf, GuardError> {
        let output = self
            .query(
                ["rev-parse", "--path-format=absolute", "--git-path", name],
                REMOVAL_LIMITS,
            )
            .await?;
        Ok(PathBuf::from(String::from_utf8_lossy(&output).trim_end()))
    }

    pub async fn commits_ahead(&self, branch: &str) -> Result<u32, GuardError> {
        let range = format!("{}..refs/heads/{branch}", self.base_commit);
        let output = self
            .query(
                ["rev-list", "--count", "--end-of-options", &range],
                REMOVAL_LIMITS,
            )
            .await?;
        String::from_utf8_lossy(&output)
            .trim()
            .parse()
            .map_err(|_| GuardError::Failed(None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::worktree::simplified;
    use std::collections::BTreeMap;

    struct Fixture {
        _dir: tempfile::TempDir,
        root: PathBuf,
        repo: PathBuf,
        work_tree: PathBuf,
        marker: PathBuf,
        identity: RepositoryIdentity,
        base: String,
    }

    fn git(dir: &Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(["-c", "commit.gpgsign=false"])
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {args:?}");
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn slashed(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    fn fixture(files: &[(&str, &str)]) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let root = simplified(std::fs::canonicalize(dir.path()).unwrap());
        let repo = root.join("repo");
        std::fs::create_dir(&repo).unwrap();
        git(&repo, &["init", "-q", "--initial-branch=main"]);
        for (name, content) in [("a.txt", "one\n"), ("b.bin", "bin\n")]
            .into_iter()
            .chain(files.iter().copied())
        {
            std::fs::write(repo.join(name), content).unwrap();
        }
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "-q", "-m", "init"]);
        let work_tree = root.join("wt");
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "meelting/x",
                work_tree.to_str().unwrap(),
                "HEAD",
            ],
        );
        let common = git(
            &repo,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        );
        Fixture {
            marker: root.join("marker"),
            identity: RepositoryIdentity {
                id: "r".into(),
                name: "repo".into(),
                path: repo.to_string_lossy().into_owned(),
                git_common_dir: common,
            },
            base: git(&repo, &["rev-parse", "HEAD"]),
            _dir: dir,
            root,
            repo,
            work_tree,
        }
    }

    impl Fixture {
        fn probe(&self, name: &str) -> String {
            format!("echo {name} >> '{}'; cat", slashed(&self.marker))
        }

        fn quiet_probe(&self, name: &str) -> String {
            format!("echo {name} >> '{}'; true", slashed(&self.marker))
        }

        fn executed(&self) -> String {
            std::fs::read_to_string(&self.marker).unwrap_or_default()
        }

        fn clear(&self) {
            let _ = std::fs::remove_file(&self.marker);
        }

        fn config(&self, key: &str, value: &str) {
            git(&self.repo, &["config", key, value]);
        }

        async fn guard(&self) -> Result<Guard, GuardError> {
            Guard::open(&self.identity, &self.work_tree, &self.base).await
        }

        fn dirty(&self) {
            std::fs::write(self.work_tree.join("a.txt"), "two\n").unwrap();
        }

        fn plain_status_executes(&self) -> bool {
            self.clear();
            git(&self.work_tree, &["status", "--porcelain"]);
            let executed = !self.executed().is_empty();
            self.clear();
            executed
        }

        fn admin(&self) -> PathBuf {
            admin_dir(Path::new(&self.identity.git_common_dir), &self.work_tree).unwrap()
        }
    }

    fn copy_all(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap().flatten() {
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_all(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut files = BTreeMap::new();
        let mut pending = vec![dir.to_path_buf()];
        while let Some(folder) = pending.pop() {
            for entry in std::fs::read_dir(&folder).unwrap().flatten() {
                if entry.file_type().unwrap().is_dir() {
                    pending.push(entry.path());
                } else {
                    let relative = entry.path().strip_prefix(dir).unwrap().to_path_buf();
                    files.insert(relative, std::fs::read(entry.path()).unwrap());
                }
            }
        }
        files
    }

    const DIFF: [&str; 5] = [
        "diff",
        "--no-ext-diff",
        "--no-textconv",
        "--no-color",
        "--ignore-submodules=all",
    ];

    #[tokio::test]
    async fn a_redirected_git_file_cannot_change_which_repository_is_queried() {
        let fx = fixture(&[(".gitattributes", "*.txt filter=probe\n")]);
        let hostile = fx.root.join("hostile");
        let common = PathBuf::from(&fx.identity.git_common_dir);
        copy_all(&common, &hostile);
        git(
            &hostile,
            &["config", "filter.probe.clean", &fx.probe("hostile-clean")],
        );
        let guard = fx.guard().await.unwrap();
        let name = fx.admin().file_name().unwrap().to_owned();
        std::fs::write(
            fx.work_tree.join(".git"),
            format!(
                "gitdir: {}\n",
                slashed(&hostile.join("worktrees").join(name))
            ),
        )
        .unwrap();
        fx.dirty();

        let status = guard.status_exhaustive().await.unwrap();

        assert!(String::from_utf8_lossy(&status).contains("a.txt"));
        assert_eq!(fx.executed(), "");
        assert!(
            fx.plain_status_executes(),
            "the setup must be able to run the filter"
        );
    }

    #[tokio::test]
    async fn rejects_an_executable_filter_that_applies_to_the_base_commit() {
        let definitions: [(&str, &str, Option<String>); 4] = [
            ("repository config", "filter.probe.clean", None),
            ("process filter", "filter.probe.process", None),
            ("include.path", "include.path", Some("include".into())),
            (
                "includeIf",
                "includeIf.__root__.path",
                Some("conditional".into()),
            ),
        ];
        for (label, key, include) in definitions {
            let fx = fixture(&[(".gitattributes", "*.txt filter=probe\n")]);
            let command = fx.probe("clean");
            match include {
                None => fx.config(key, &command),
                Some(name) => {
                    let file = fx.root.join(format!("{name}.cfg"));
                    std::fs::write(&file, format!("[filter \"probe\"]\n\tclean = {command}\n"))
                        .unwrap();
                    let key = key.replace("__root__", &format!("gitdir:{}/", slashed(&fx.root)));
                    fx.config(&key, &slashed(&file));
                }
            }

            let result = fx.guard().await;

            assert!(
                matches!(&result, Err(GuardError::FiltersApply(names)) if names == &["probe"]),
                "{label}: {result:?}"
            );
            assert_eq!(fx.executed(), "", "{label}");
        }
    }

    #[tokio::test]
    async fn rejects_a_filter_defined_for_the_worktree_only() {
        let fx = fixture(&[(".gitattributes", "*.txt filter=probe\n")]);
        git(&fx.repo, &["config", "extensions.worktreeConfig", "true"]);
        git(
            &fx.work_tree,
            &[
                "config",
                "--worktree",
                "filter.probe.clean",
                &fx.probe("clean"),
            ],
        );

        assert!(matches!(fx.guard().await, Err(GuardError::FiltersApply(_))));
        assert_eq!(fx.executed(), "");
    }

    #[tokio::test]
    async fn defined_but_unused_filters_do_not_block_the_query() {
        let fx = fixture(&[(".gitattributes", "*.txt text\n")]);
        fx.config("filter.lfs.clean", &fx.probe("clean"));
        fx.dirty();

        let guard = fx.guard().await.unwrap();
        let status = guard.status_exhaustive().await.unwrap();

        assert!(String::from_utf8_lossy(&status).contains("a.txt"));
        assert_eq!(fx.executed(), "");
        guard.finish().await.unwrap();
    }

    #[tokio::test]
    async fn attributes_written_in_the_worktree_never_select_a_filter() {
        let fx = fixture(&[]);
        fx.config("filter.probe.clean", &fx.probe("clean"));
        std::fs::write(fx.work_tree.join(".gitattributes"), "*.txt filter=probe\n").unwrap();
        std::fs::create_dir(fx.work_tree.join("nested")).unwrap();
        std::fs::write(
            fx.work_tree.join("nested/.gitattributes"),
            "* filter=probe\n",
        )
        .unwrap();
        std::fs::write(fx.work_tree.join("nested/x.txt"), "x\n").unwrap();
        fx.dirty();

        let guard = fx.guard().await.unwrap();
        guard.status_exhaustive().await.unwrap();
        guard.query(DIFF, REMOVAL_LIMITS).await.unwrap();

        assert_eq!(fx.executed(), "");
        assert!(
            fx.plain_status_executes(),
            "the setup must be able to run the filter"
        );
    }

    #[tokio::test]
    async fn info_attributes_must_be_comments_and_a_regular_file() {
        let fx = fixture(&[]);
        let info = PathBuf::from(&fx.identity.git_common_dir).join("info");
        std::fs::create_dir_all(&info).unwrap();
        std::fs::write(info.join("attributes"), "# only a note\n\n").unwrap();
        assert!(fx.guard().await.is_ok());

        std::fs::write(info.join("attributes"), "*.txt filter=probe\n").unwrap();
        assert!(matches!(
            fx.guard().await,
            Err(GuardError::InfoAttributesUnsafe)
        ));

        std::fs::remove_file(info.join("attributes")).unwrap();
        #[cfg(unix)]
        {
            let target = fx.root.join("elsewhere");
            std::fs::write(&target, "# note\n").unwrap();
            std::os::unix::fs::symlink(&target, info.join("attributes")).unwrap();
            assert!(matches!(
                fx.guard().await,
                Err(GuardError::InfoAttributesUnsafe)
            ));
        }
        std::fs::remove_file(info.join("attributes")).ok();
        std::fs::create_dir(info.join("attributes")).unwrap();
        assert!(matches!(
            fx.guard().await,
            Err(GuardError::InfoAttributesUnsafe)
        ));
    }

    #[tokio::test]
    async fn a_configuration_added_after_the_check_is_detected_and_not_applied() {
        let fx = fixture(&[]);
        fx.dirty();
        let guard = fx.guard().await.unwrap();
        fx.config("filter.probe.clean", &fx.probe("clean"));
        std::fs::write(fx.work_tree.join(".gitattributes"), "*.txt filter=probe\n").unwrap();

        let result = guard.status_exhaustive().await;

        assert!(matches!(result, Err(GuardError::ConfigChanged)));
        assert_eq!(
            fx.executed(),
            "",
            "the pinned attributes ignore the new assignment"
        );
        assert!(matches!(
            guard.finish().await,
            Err(GuardError::ConfigChanged)
        ));
    }

    #[tokio::test]
    async fn textconv_and_external_diff_do_not_run() {
        let fx = fixture(&[(".gitattributes", "*.bin diff=probe\n")]);
        fx.config("diff.probe.textconv", &fx.probe("textconv"));
        fx.config("diff.probe.command", &fx.quiet_probe("driver"));
        fx.config("diff.external", &fx.quiet_probe("external"));
        std::fs::write(fx.work_tree.join("b.bin"), "new\n").unwrap();
        git(&fx.work_tree, &["add", "b.bin"]);
        std::fs::write(fx.work_tree.join("b.bin"), "newer\n").unwrap();
        let guard = fx.guard().await.unwrap();

        for staged in [false, true] {
            let mut args = DIFF.to_vec();
            if staged {
                args.push("--cached");
            }
            let patch = guard.query(args, REMOVAL_LIMITS).await.unwrap();
            assert!(String::from_utf8_lossy(&patch).contains("b.bin"));
        }
        guard
            .query(
                ["log", "-p", "-1", "--no-ext-diff", "--no-textconv"],
                REMOVAL_LIMITS,
            )
            .await
            .unwrap();

        assert_eq!(fx.executed(), "");
    }

    #[tokio::test]
    async fn hooks_and_fsmonitor_do_not_run() {
        let fx = fixture(&[]);
        let hooks = fx.root.join("hooks");
        std::fs::create_dir(&hooks).unwrap();
        for name in [
            "post-index-change",
            "fsmonitor-watchman",
            "reference-transaction",
        ] {
            let hook = hooks.join(name);
            std::fs::write(
                &hook,
                format!("#!/bin/sh\necho {name} >> '{}'\n", slashed(&fx.marker)),
            )
            .unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
        fx.config("core.hooksPath", &slashed(&hooks));
        fx.config("core.fsmonitor", &fx.quiet_probe("fsmonitor"));
        fx.dirty();
        std::fs::write(fx.work_tree.join("b.bin"), "bin\n").unwrap();
        let guard = fx.guard().await.unwrap();

        guard.status_exhaustive().await.unwrap();
        guard.query(DIFF, REMOVAL_LIMITS).await.unwrap();

        assert_eq!(fx.executed(), "");
        fx.config("core.fsmonitor", "true");
        guard.status_exhaustive().await.unwrap_err();
        let watching = std::process::Command::new("git")
            .arg("-C")
            .arg(&fx.work_tree)
            .args(["fsmonitor--daemon", "status"])
            .output()
            .unwrap();
        let watching = String::from_utf8_lossy(&watching.stdout);
        assert!(!watching.contains("is watching"), "{watching}");
    }

    #[tokio::test]
    async fn gitlinks_are_reported_and_never_entered() {
        let fx = fixture(&[]);
        let nested = fx.work_tree.join("nested");
        std::fs::create_dir(&nested).unwrap();
        git(&nested, &["init", "-q"]);
        git(
            &nested,
            &["config", "core.fsmonitor", &fx.quiet_probe("nested")],
        );
        git(
            &nested,
            &[
                "-c",
                "core.fsmonitor=false",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                "n",
            ],
        );
        git(&fx.work_tree, &["add", "nested"]);
        std::fs::write(nested.join("dirty"), "x").unwrap();
        fx.clear();
        let guard = fx.guard().await.unwrap();

        let status = guard.status_exhaustive().await.unwrap();
        let gitlinks = guard.gitlinks().await.unwrap();

        assert_eq!(gitlinks, ["nested"]);
        assert!(!String::from_utf8_lossy(&status).contains("nested"));
        assert_eq!(fx.executed(), "");
    }

    #[tokio::test]
    async fn read_queries_leave_the_repository_untouched() {
        let fx = fixture(&[(".gitattributes", "*.txt text eol=lf\n")]);
        fx.dirty();
        std::fs::write(fx.work_tree.join("b.bin"), "bin\n").unwrap();
        std::fs::write(fx.work_tree.join("untracked.txt"), "u\n").unwrap();
        let common = PathBuf::from(&fx.identity.git_common_dir);
        let before = (snapshot(&common), snapshot(&fx.admin()));
        let guard = fx.guard().await.unwrap();

        guard.status_exhaustive().await.unwrap();
        guard.query(DIFF, REMOVAL_LIMITS).await.unwrap();
        let mut cached = DIFF.to_vec();
        cached.push("--cached");
        guard.query(cached, REMOVAL_LIMITS).await.unwrap();
        guard
            .query(["ls-files", "-m"], REMOVAL_LIMITS)
            .await
            .unwrap();
        guard.head_reference().await.unwrap();
        guard.commits_ahead("meelting/x").await.unwrap();
        guard.git_path("index.lock").await.unwrap();
        guard.finish().await.unwrap();

        assert!(before == (snapshot(&common), snapshot(&fx.admin())));
    }

    #[tokio::test]
    async fn line_ending_conversions_do_not_create_false_changes() {
        let fx = fixture(&[(".gitattributes", "*.txt text eol=crlf\n")]);
        std::fs::remove_file(fx.work_tree.join("a.txt")).unwrap();
        git(&fx.work_tree, &["checkout", "-q", "--", "a.txt"]);
        assert!(
            std::fs::read(fx.work_tree.join("a.txt"))
                .unwrap()
                .contains(&b'\r')
        );
        let guard = fx.guard().await.unwrap();

        let status = guard.status_exhaustive().await.unwrap();

        assert!(status.is_empty(), "{}", String::from_utf8_lossy(&status));
    }

    #[tokio::test]
    async fn only_allow_listed_conversion_values_reach_git() {
        let fx = fixture(&[]);
        fx.config("core.autocrlf", "INPUT");
        fx.config("core.eol", "lf; touch /tmp/x");
        let guard = fx.guard().await.unwrap();

        assert_eq!(guard.conversions, [("core.autocrlf", "input".to_string())]);
    }

    #[tokio::test]
    async fn output_beyond_the_limit_is_cut_and_never_reported_as_complete() {
        let fx = fixture(&[]);
        for index in 0..40 {
            std::fs::write(fx.work_tree.join(format!("untracked-{index}.txt")), "x").unwrap();
        }
        let guard = fx.guard().await.unwrap();
        let small = Limits {
            timeout: Duration::from_secs(30),
            max_stdout: 100,
        };
        let status = ["status", "--porcelain=v2", "-z", "--untracked-files=all"];

        let partial = guard.query_partial(status, small).await.unwrap();
        let strict = guard.query(status, small).await;

        assert!(partial.truncated && partial.stdout.len() == 100);
        assert!(matches!(strict, Err(GuardError::OutputTooLarge)));
    }

    #[tokio::test]
    async fn a_slow_git_is_stopped_at_the_timeout() {
        let mut slow = command(true);
        slow.args(["-c", "alias.slow=!sleep 20", "slow"]);
        let started = std::time::Instant::now();

        let result = execute(
            slow,
            Limits {
                timeout: Duration::from_millis(300),
                max_stdout: 1024,
            },
        )
        .await;

        assert!(matches!(result, Err(GuardError::TimedOut)));
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn every_inherited_git_variable_is_dropped_in_any_letter_case() {
        let names = [
            "GIT_DIR",
            "git_trace",
            "Git_External_Diff",
            "PATH",
            "HOME",
            "GITHUB_TOKEN",
        ]
        .map(OsString::from);

        let dropped = inherited_git_variables(names.into_iter());

        assert_eq!(dropped, ["GIT_DIR", "git_trace", "Git_External_Diff"]);
    }

    #[test]
    fn the_admin_dir_comes_from_the_trusted_registry_not_from_the_worktree() {
        let fx = fixture(&[]);
        let common = PathBuf::from(&fx.identity.git_common_dir);
        let expected = fx.admin();
        let roundabout = fx.work_tree.join("..").join("wt");

        assert_eq!(admin_dir(&common, &roundabout).unwrap(), expected);
        assert!(matches!(
            admin_dir(&common, &fx.repo),
            Err(GuardError::GitDirNotFound)
        ));
        #[cfg(windows)]
        {
            let verbatim = std::fs::canonicalize(&fx.work_tree).unwrap();
            assert_eq!(admin_dir(&common, &verbatim).unwrap(), expected);
        }
        std::fs::remove_file(fx.work_tree.join(".git")).unwrap();
        assert_eq!(admin_dir(&common, &fx.work_tree).unwrap(), expected);
    }

    #[test]
    fn a_symbolic_link_in_the_registry_is_not_followed() {
        #[cfg(unix)]
        {
            let fx = fixture(&[]);
            let common = PathBuf::from(&fx.identity.git_common_dir);
            let admin = fx.admin();
            let moved = fx.root.join("moved-admin");
            std::fs::rename(&admin, &moved).unwrap();
            std::os::unix::fs::symlink(&moved, &admin).unwrap();

            assert!(matches!(
                admin_dir(&common, &fx.work_tree),
                Err(GuardError::GitDirNotFound)
            ));
        }
    }
}
