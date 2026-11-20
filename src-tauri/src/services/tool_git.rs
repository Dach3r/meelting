use crate::domain::repository::RepositoryIdentity;
use crate::domain::review::parse_review_status;
use crate::domain::tools::{Request, render_diff, render_status};
use crate::domain::worktree::Worktree;
use crate::services::environment_review::{guarded, review_error, unchanged_since};
use crate::services::repository_locks::RepositoryLocks;
use crate::services::session_load::SessionError;
use crate::services::tool_files::ToolResult;
use crate::services::worktree_remove::verify_identity;
use crate::system::git_guarded::GuardError;
use crate::system::storage;

fn code(error: SessionError) -> &'static str {
    match error {
        SessionError::GitUnavailable => "git_unavailable",
        SessionError::GitTooOld => "git_too_old",
        SessionError::ProcessIsolationUnavailable => "isolation_unavailable",
        SessionError::EnvironmentFiltersApply | SessionError::EnvironmentConfigUnsafe => {
            "environment_config_unsafe"
        }
        SessionError::ReviewTimeout => "git_timeout",
        SessionError::DiffTooLarge => "output_too_large",
        SessionError::ReviewStale => "environment_changed",
        SessionError::EnvironmentIdentityMismatch | SessionError::RepositoryMoved => {
            "environment_mismatch"
        }
        _ => "environment_unavailable",
    }
}

fn guard_code(error: GuardError) -> &'static str {
    code(review_error(error))
}

pub async fn run(
    locks: &RepositoryLocks,
    worktree: &Worktree,
    repository: &RepositoryIdentity,
    request: &Request,
) -> ToolResult {
    let _lock = locks.acquire(&repository.git_common_dir).await;
    verify_identity(repository, worktree, &storage::check)
        .await
        .map_err(code)?;
    let guard = guarded(repository, worktree).await.map_err(code)?;
    let state = guard.review_state().await.map_err(guard_code)?;
    let result = match request {
        Request::GitStatus => {
            let output = guard.review_status().await.map_err(guard_code)?;
            let status = parse_review_status(&output.stdout, output.truncated);
            render_status(&status.entries, status.complete)
        }
        Request::GitDiff { source, path } => {
            let output = guard
                .diff_file(*source, path.as_bytes(), None)
                .await
                .map_err(guard_code)?;
            render_diff(&output.stdout, output.truncated)
        }
        _ => return Err("internal_error"),
    };
    unchanged_since(&guard, &state).await.map_err(code)?;
    guard.finish().await.map_err(guard_code)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::review::DiffSource;
    use crate::domain::tools::{MAX_DIFF_BYTES, Prepared, prepare};
    use crate::services::environment_review::tests::{Environment, environment};
    use crate::services::repository_register::tests::git;
    use crate::services::worktree_remove::ready_environment;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    async fn call(
        env: &Environment,
        name: &str,
        arguments: &str,
    ) -> crate::services::tool_files::ToolResult {
        let Prepared::Audited { request, .. } = prepare(name, arguments, false) else {
            panic!("unauditable");
        };
        let request = request.unwrap();
        let (_, worktree, repository) = ready_environment(&env.fixture.pool, &env.task_id)
            .await
            .unwrap();
        run(&env.fixture.locks, &worktree, &repository, &request).await
    }

    async fn status(env: &Environment) -> String {
        call(env, "git_status", "{}").await.unwrap().0
    }

    fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if let Ok(bytes) = fs::read(&path) {
                    out.insert(path, bytes);
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, &mut out);
        out
    }

    #[tokio::test]
    async fn status_lists_staged_unstaged_and_untracked_files_and_hides_secrets() {
        let env = environment().await;
        assert_eq!(status(&env).await, "working tree clean\n");
        fs::write(env.path.join("README.md"), "changed\n").unwrap();
        fs::write(env.path.join("added.txt"), "new\n").unwrap();
        git(&env.path, &["add", "added.txt"]);
        fs::write(env.path.join("loose.txt"), "x\n").unwrap();
        fs::write(env.path.join(".env"), "TOKEN=1\n").unwrap();

        let text = status(&env).await;

        assert!(text.contains(".M README.md\n"), "{text}");
        assert!(text.contains("A. added.txt\n"));
        assert!(text.contains("?? loose.txt\n"));
        assert!(text.contains("[1 protected entries hidden]"));
        assert!(!text.contains(".env"));
    }

    #[tokio::test]
    async fn diff_shows_unstaged_staged_and_branch_changes_of_one_file() {
        let env = environment().await;
        fs::write(env.path.join("README.md"), "hello\nstaged line\n").unwrap();
        git(&env.path, &["add", "README.md"]);
        fs::write(
            env.path.join("README.md"),
            "hello\nstaged line\nworking line\n",
        )
        .unwrap();

        let unstaged = call(&env, "git_diff", r#"{"path":"README.md"}"#)
            .await
            .unwrap();
        let staged = call(
            &env,
            "git_diff",
            r#"{"path":"README.md","source":"staged"}"#,
        )
        .await
        .unwrap();
        git(&env.path, &["commit", "-am", "work"]);
        let branch = call(
            &env,
            "git_diff",
            r#"{"path":"README.md","source":"branch"}"#,
        )
        .await
        .unwrap();
        let none = call(&env, "git_diff", r#"{"path":"README.md"}"#)
            .await
            .unwrap();

        assert!(unstaged.0.contains("+++ b/README.md") && unstaged.0.contains("+working line"));
        assert!(!unstaged.0.contains("+staged line"));
        assert!(staged.0.contains("+staged line") && !staged.0.contains("+working line"));
        assert!(branch.0.contains("+staged line") && branch.0.contains("+working line"));
        assert_eq!(none, ("no differences\n".into(), false));
        assert!(!unstaged.1 && !staged.1 && !branch.1);
    }

    #[tokio::test]
    async fn a_large_diff_is_cut_at_32_kib_and_says_so() {
        let env = environment().await;
        fs::write(env.path.join("README.md"), "line\n".repeat(30_000)).unwrap();

        let (text, truncated) = call(&env, "git_diff", r#"{"path":"README.md"}"#)
            .await
            .unwrap();

        assert!(truncated);
        assert!(text.len() <= MAX_DIFF_BYTES);
        assert!(text.ends_with("[truncated]\n"));
    }

    #[tokio::test]
    async fn the_git_tools_change_nothing_in_the_worktree_the_index_or_the_repository() {
        let env = environment().await;
        fs::write(env.path.join("README.md"), "dirty\n").unwrap();
        fs::write(env.path.join("loose.txt"), "x\n").unwrap();
        let admin = {
            let pointer = fs::read_to_string(env.path.join(".git")).unwrap();
            PathBuf::from(pointer.trim().trim_start_matches("gitdir: "))
        };
        let before = (
            snapshot(&env.path),
            snapshot(&admin),
            snapshot(&env.fixture.repo.join(".git")),
        );

        status(&env).await;
        call(&env, "git_diff", r#"{"path":"README.md"}"#)
            .await
            .unwrap();
        call(
            &env,
            "git_diff",
            r#"{"path":"README.md","source":"branch"}"#,
        )
        .await
        .unwrap();

        let after = (
            snapshot(&env.path),
            snapshot(&admin),
            snapshot(&env.fixture.repo.join(".git")),
        );
        assert!(before == after);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn configured_programs_never_run() {
        use std::os::unix::fs::PermissionsExt;
        let env = environment().await;
        let marker = env.path.parent().unwrap().join("marker");
        let script = env.path.parent().unwrap().join("script.sh");
        fs::write(
            &script,
            format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
        )
        .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        let script = script.to_string_lossy().to_string();
        let repo = &env.fixture.repo;
        for (key, value) in [
            ("diff.external", script.as_str()),
            ("core.fsmonitor", script.as_str()),
            ("core.pager", script.as_str()),
            ("diff.textconv-test.textconv", script.as_str()),
        ] {
            git(repo, &["config", key, value]);
        }
        fs::write(env.path.join("README.md"), "changed\n").unwrap();

        let diff = call(&env, "git_diff", r#"{"path":"README.md"}"#).await;
        let state = status(&env).await;

        assert!(diff.unwrap().0.contains("+changed"));
        assert!(state.contains("README.md"));
        assert!(!marker.exists());
    }

    #[tokio::test]
    async fn an_unsafe_repository_configuration_is_refused_without_falling_back_to_plain_git() {
        let env = environment().await;
        let extra = env.path.parent().unwrap().join("extra.config");
        fs::write(&extra, "[user]\n\tname = x\n").unwrap();
        git(
            &env.fixture.repo,
            &["config", "include.path", &extra.to_string_lossy()],
        );

        let result = call(&env, "git_status", "{}").await;

        assert_eq!(result, Err("environment_config_unsafe"));
    }

    #[tokio::test]
    async fn an_environment_that_left_its_branch_is_refused() {
        let env = environment().await;
        git(&env.path, &["checkout", "-b", "elsewhere"]);

        assert_eq!(
            call(&env, "git_status", "{}").await,
            Err("environment_mismatch")
        );
        assert_eq!(
            call(&env, "git_diff", r#"{"path":"README.md"}"#).await,
            Err("environment_mismatch")
        );
    }

    #[test]
    fn every_failure_becomes_a_fixed_code_that_carries_no_git_output() {
        let cases = [
            (GuardError::GitTooOld, "git_too_old"),
            (GuardError::IsolationUnavailable, "isolation_unavailable"),
            (
                GuardError::FiltersApply(vec!["secret-filter".into()]),
                "environment_config_unsafe",
            ),
            (
                GuardError::InfoAttributesUnsafe,
                "environment_config_unsafe",
            ),
            (GuardError::ConfigIncludes, "environment_config_unsafe"),
            (GuardError::ConfigChanged, "environment_config_unsafe"),
            (GuardError::FilterBlocked, "environment_config_unsafe"),
            (GuardError::TimedOut, "git_timeout"),
            (GuardError::OutputTooLarge, "output_too_large"),
            (GuardError::GitDirNotFound, "environment_mismatch"),
            (GuardError::Failed(Some(128)), "environment_unavailable"),
            (
                GuardError::Unavailable(std::io::Error::other("fatal: /home/user/secret")),
                "git_unavailable",
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(guard_code(error), expected);
        }
        assert_eq!(code(SessionError::ReviewStale), "environment_changed");
        assert_eq!(
            code(SessionError::ProcessIsolationUnavailable),
            "isolation_unavailable"
        );
    }

    #[test]
    fn the_diff_source_is_a_closed_enum_never_a_revision_or_an_option() {
        for source in ["HEAD~1", "--output=/tmp/x", "origin/main", "main..HEAD"] {
            let arguments = format!(r#"{{"path":"a","source":"{source}"}}"#);
            let Prepared::Audited { request, .. } = prepare("git_diff", &arguments, false) else {
                panic!("unauditable");
            };
            assert_eq!(request, Err("invalid_arguments"));
        }
        let _ = DiffSource::Branch;
    }
}
