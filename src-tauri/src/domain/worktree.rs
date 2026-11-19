use crate::domain::storage::StorageIssue;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorktreesRoot {
    pub path: String,
    pub custom: bool,
    pub issue: Option<StorageIssue>,
}

pub fn default_root(home: &Path) -> PathBuf {
    home.join("Meelting").join("worktrees")
}

pub fn simplified(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => path,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeStatus {
    Preparing,
    Ready,
    Failed,
    Missing,
    Removing,
    Removed,
}

impl WorktreeStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            WorktreeStatus::Preparing => "preparing",
            WorktreeStatus::Ready => "ready",
            WorktreeStatus::Failed => "failed",
            WorktreeStatus::Missing => "missing",
            WorktreeStatus::Removing => "removing",
            WorktreeStatus::Removed => "removed",
        }
    }

    pub fn parse(value: &str) -> Option<WorktreeStatus> {
        match value {
            "preparing" => Some(WorktreeStatus::Preparing),
            "ready" => Some(WorktreeStatus::Ready),
            "failed" => Some(WorktreeStatus::Failed),
            "missing" => Some(WorktreeStatus::Missing),
            "removing" => Some(WorktreeStatus::Removing),
            "removed" => Some(WorktreeStatus::Removed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeErrorCode {
    CheckoutFailed,
    VerificationFailed,
    PreparationInterrupted,
    RemovalInterrupted,
}

impl WorktreeErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            WorktreeErrorCode::CheckoutFailed => "checkout_failed",
            WorktreeErrorCode::VerificationFailed => "verification_failed",
            WorktreeErrorCode::PreparationInterrupted => "preparation_interrupted",
            WorktreeErrorCode::RemovalInterrupted => "removal_interrupted",
        }
    }

    pub fn parse(value: &str) -> Option<WorktreeErrorCode> {
        match value {
            "checkout_failed" => Some(WorktreeErrorCode::CheckoutFailed),
            "verification_failed" => Some(WorktreeErrorCode::VerificationFailed),
            "preparation_interrupted" => Some(WorktreeErrorCode::PreparationInterrupted),
            "removal_interrupted" => Some(WorktreeErrorCode::RemovalInterrupted),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Worktree {
    pub id: String,
    pub path: String,
    pub branch_name: String,
    pub base_ref: String,
    pub base_commit_sha: String,
    pub status: WorktreeStatus,
    pub error_code: Option<WorktreeErrorCode>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredWorktree {
    pub path: String,
    pub head: Option<String>,
    pub branch: Option<String>,
}

pub fn git_supports_attribute_source(version: &str) -> bool {
    let numbers: Vec<u32> = version
        .trim()
        .trim_start_matches("git version ")
        .split(['.', ' '])
        .take(2)
        .map_while(|part| part.parse().ok())
        .collect();
    matches!(numbers[..], [major, minor] if (major, minor) >= (2, 40))
}

pub const SLUG_MAX_CHARS: usize = 40;

pub fn slug(text: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    let mut slug = String::new();
    for character in text.nfd().filter(char::is_ascii) {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let slug: String = slug.chars().take(SLUG_MAX_CHARS).collect();
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "task".to_string()
    } else {
        slug.to_string()
    }
}

pub fn random_suffix(uuid: &str) -> &str {
    &uuid[uuid.len().saturating_sub(12)..]
}

pub fn branch_name(title: &str, worktree_id: &str) -> String {
    format!("meelting/{}-{}", slug(title), random_suffix(worktree_id))
}

pub fn worktree_folder(
    root: &Path,
    repository_name: &str,
    repository_id: &str,
    title: &str,
    worktree_id: &str,
) -> (PathBuf, PathBuf) {
    let repository_folder = root.join(format!(
        "{}--{}",
        slug(repository_name),
        random_suffix(repository_id)
    ));
    let folder = repository_folder.join(format!("{}-{}", slug(title), random_suffix(worktree_id)));
    (repository_folder, folder)
}

pub fn parse_worktree_list(porcelain: &str) -> Vec<RegisteredWorktree> {
    let mut entries = Vec::new();
    let mut current: Option<RegisteredWorktree> = None;
    for field in porcelain.split('\0') {
        if field.is_empty() {
            entries.extend(current.take());
        } else if let Some(path) = field.strip_prefix("worktree ") {
            entries.extend(current.take());
            current = Some(RegisteredWorktree {
                path: path.to_string(),
                head: None,
                branch: None,
            });
        } else if let Some(entry) = current.as_mut() {
            if let Some(head) = field.strip_prefix("HEAD ") {
                entry.head = Some(head.to_string());
            } else if let Some(branch) = field.strip_prefix("branch refs/heads/") {
                entry.branch = Some(branch.to_string());
            }
        }
    }
    entries.extend(current);
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_a_folder_in_the_home() {
        assert_eq!(
            default_root(Path::new("/Users/ana")),
            Path::new("/Users/ana/Meelting/worktrees")
        );
    }

    #[test]
    fn drops_the_windows_verbatim_prefix_only_for_drive_paths() {
        assert_eq!(
            simplified(PathBuf::from(r"\\?\C:\Users\ana")),
            PathBuf::from(r"C:\Users\ana")
        );
        assert_eq!(
            simplified(PathBuf::from(r"\\?\UNC\server\share")),
            PathBuf::from(r"\\?\UNC\server\share")
        );
        assert_eq!(
            simplified(PathBuf::from("/Users/ana")),
            PathBuf::from("/Users/ana")
        );
    }

    #[test]
    fn statuses_and_error_codes_round_trip() {
        use WorktreeStatus::*;
        for status in [Preparing, Ready, Failed, Missing, Removing, Removed] {
            assert_eq!(WorktreeStatus::parse(status.as_str()), Some(status));
        }
        use WorktreeErrorCode::*;
        for code in [
            CheckoutFailed,
            VerificationFailed,
            PreparationInterrupted,
            RemovalInterrupted,
        ] {
            assert_eq!(WorktreeErrorCode::parse(code.as_str()), Some(code));
            assert!(
                code.as_str()
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_')
            );
        }
        assert_eq!(WorktreeStatus::parse("done"), None);
    }

    #[test]
    fn slugs_are_short_ascii_and_never_empty() {
        assert_eq!(slug("Fix the Login Timeout!"), "fix-the-login-timeout");
        assert_eq!(slug("Migración de pagos ñandú"), "migracion-de-pagos-nandu");
        assert_eq!(slug("  --  "), "task");
        assert_eq!(slug("日本語"), "task");
        assert_eq!(slug(&"a b ".repeat(40)).chars().count(), SLUG_MAX_CHARS - 1);
        assert!(!slug(&"ab-".repeat(30)).ends_with('-'));
    }

    #[test]
    fn names_use_the_random_end_of_the_uuid() {
        let worktree_id = "019a8b2c-1d2e-7f30-8a4b-5c6d7e8f9a0b";
        let repository_id = "019a8b2c-0000-7000-9fff-aabbccddeeff";

        assert_eq!(random_suffix(worktree_id), "5c6d7e8f9a0b");
        assert_eq!(
            branch_name("Fix login", worktree_id),
            "meelting/fix-login-5c6d7e8f9a0b"
        );
        let (repository_folder, folder) = worktree_folder(
            Path::new("/w"),
            "My App",
            repository_id,
            "Fix login",
            worktree_id,
        );
        assert_eq!(repository_folder, Path::new("/w/my-app--aabbccddeeff"));
        assert_eq!(
            folder,
            Path::new("/w/my-app--aabbccddeeff/fix-login-5c6d7e8f9a0b")
        );
    }

    #[test]
    fn branch_names_fit_the_schema() {
        let name = branch_name(&"Ñ x".repeat(100), "019a8b2c-1d2e-7f30-8a4b-5c6d7e8f9a0b");

        assert!(name.len() <= 120);
        assert!(name.starts_with("meelting/"));
        assert!(
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '/' || c == '-')
        );
    }

    #[test]
    fn parses_the_porcelain_worktree_list() {
        let porcelain = "worktree /repo\0HEAD aaa\0branch refs/heads/main\0\0worktree /w/one\0HEAD bbb\0branch refs/heads/meelting/x-1\0locked\0\0worktree /w/detached\0HEAD ccc\0detached\0\0";

        let entries = parse_worktree_list(porcelain);

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[1].path, "/w/one");
        assert_eq!(entries[1].head.as_deref(), Some("bbb"));
        assert_eq!(entries[1].branch.as_deref(), Some("meelting/x-1"));
        assert_eq!(entries[2].branch, None);
    }

    #[test]
    fn requires_git_2_40_to_read_attributes_from_a_commit() {
        assert!(git_supports_attribute_source("git version 2.54.0"));
        assert!(git_supports_attribute_source(
            "git version 2.40.1.windows.1"
        ));
        assert!(git_supports_attribute_source("git version 3.0.0"));
        assert!(!git_supports_attribute_source(
            "git version 2.39.5 (Apple Git-154)"
        ));
        assert!(!git_supports_attribute_source("nonsense"));
    }
}
