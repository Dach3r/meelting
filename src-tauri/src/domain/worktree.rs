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
    pub copies: Vec<crate::domain::file_copy::FileCopy>,
    pub trash_path: Option<String>,
    pub trashed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Worktree {
    pub fn location(&self) -> &str {
        self.trash_path.as_deref().unwrap_or(&self.path)
    }
}

pub const TRASH_DAYS: u32 = 30;

pub fn trash_folder(path: &Path) -> Option<PathBuf> {
    Some(path.parent()?.join(".trash").join(path.file_name()?))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Environment {
    pub task_id: String,
    pub task_title: String,
    pub repository_name: String,
    pub worktree: Worktree,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Tracked,
    Conflicted,
    Untracked,
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PendingChange {
    pub kind: ChangeKind,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RemovalCheck {
    pub changes: Vec<PendingChange>,
    pub operation_in_progress: bool,
    pub commits_ahead: u32,
}

pub const IN_PROGRESS_MARKERS: &[&str] = &[
    "MERGE_HEAD",
    "rebase-merge",
    "rebase-apply",
    "CHERRY_PICK_HEAD",
    "REVERT_HEAD",
    "BISECT_LOG",
    "index.lock",
];

pub fn parse_status(porcelain: &[u8]) -> Vec<PendingChange> {
    let text = String::from_utf8_lossy(porcelain);
    let mut fields = text.split('\0').filter(|field| !field.is_empty());
    let mut changes = Vec::new();
    while let Some(field) = fields.next() {
        let (kind, skipped) = match field.as_bytes()[0] {
            b'1' => (ChangeKind::Tracked, 8),
            b'2' => (ChangeKind::Tracked, 9),
            b'u' => (ChangeKind::Conflicted, 10),
            b'?' => (ChangeKind::Untracked, 1),
            b'!' => (ChangeKind::Ignored, 1),
            _ => continue,
        };
        if let Some(path) = field.splitn(skipped + 1, ' ').nth(skipped) {
            changes.push(PendingChange {
                kind,
                path: path.to_string(),
            });
        }
        if field.starts_with('2') {
            fields.next();
        }
    }
    changes
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

pub const INFO_ATTRIBUTES_MAX_BYTES: usize = 64 * 1024;

pub fn info_attributes_allowed(content: &[u8]) -> bool {
    content.len() <= INFO_ATTRIBUTES_MAX_BYTES
        && content
            .split(|byte| *byte == b'\n')
            .all(|line| matches!(line.trim_ascii().first(), None | Some(b'#')))
}

pub fn parse_executable_filters(config_null: &[u8]) -> std::collections::BTreeSet<String> {
    String::from_utf8_lossy(config_null)
        .split('\0')
        .filter_map(|record| record.split_once('\n'))
        .filter(|(_, value)| !value.trim().is_empty())
        .filter_map(|(key, _)| {
            let name = key.strip_prefix("filter.")?;
            ["clean", "smudge", "process"]
                .iter()
                .find_map(|kind| name.strip_suffix(&format!(".{kind}")))
                .map(String::from)
        })
        .collect()
}

pub fn has_config_includes(config_null: &[u8]) -> bool {
    String::from_utf8_lossy(config_null)
        .split('\0')
        .map(|record| record.split_once('\n').map_or(record, |(key, _)| key))
        .map(str::to_ascii_lowercase)
        .any(|key| {
            key == "include.path" || (key.starts_with("includeif.") && key.ends_with(".path"))
        })
}

pub fn parse_gitlinks(stage_listing: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(stage_listing)
        .split('\0')
        .filter_map(|record| record.split_once('\t'))
        .filter(|(info, _)| info.starts_with("160000 "))
        .map(|(_, path)| path.to_string())
        .collect()
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
    fn trashes_into_a_hidden_sibling_folder_on_the_same_volume() {
        assert_eq!(
            trash_folder(Path::new("/w/my-app--aabbccddeeff/fix-login-5c6d7e8f9a0b")),
            Some(PathBuf::from(
                "/w/my-app--aabbccddeeff/.trash/fix-login-5c6d7e8f9a0b"
            ))
        );
        assert_eq!(trash_folder(Path::new("/")), None);
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

    #[test]
    fn parses_every_kind_of_pending_change_including_spaces_and_renames() {
        let porcelain = b"1 .M N... 100644 100644 100644 aaa aaa src/my file.rs\x002 R. N... 100644 100644 100644 aaa bbb R100 new name.rs\x00old name.rs\x00u UU N... 100644 100644 100644 100644 a b c conflict.rs\x00? notes.txt\x00! .env\x00! node_modules/\x00";

        let changes = parse_status(porcelain);

        let summary: Vec<(ChangeKind, &str)> = changes
            .iter()
            .map(|change| (change.kind, change.path.as_str()))
            .collect();
        assert_eq!(
            summary,
            [
                (ChangeKind::Tracked, "src/my file.rs"),
                (ChangeKind::Tracked, "new name.rs"),
                (ChangeKind::Conflicted, "conflict.rs"),
                (ChangeKind::Untracked, "notes.txt"),
                (ChangeKind::Ignored, ".env"),
                (ChangeKind::Ignored, "node_modules/"),
            ]
        );
        assert!(parse_status(b"").is_empty());
    }
    #[test]
    fn info_attributes_must_be_empty_or_only_comments() {
        assert!(info_attributes_allowed(b""));
        assert!(info_attributes_allowed(b"# note\n\n   \n\t# other\r\n"));
        assert!(!info_attributes_allowed(b"*.txt filter=lfs\n"));
        assert!(!info_attributes_allowed(b"# note\n* text=auto\n"));
        assert!(!info_attributes_allowed(&vec![
            b'#';
            INFO_ATTRIBUTES_MAX_BYTES + 1
        ]));
    }

    #[test]
    fn reads_the_filters_that_have_an_executable_driver() {
        let config = b"filter.lfs.clean\ngit-lfs clean -- %f\0filter.lfs.required\ntrue\0\
            filter.a.b.process\nrun\0filter.empty.smudge\n  \0filter.x.smudge\ncat\0diff.t.textconv\nx";

        let names: Vec<String> = parse_executable_filters(config).into_iter().collect();

        assert_eq!(names, ["a.b", "lfs", "x"]);
        assert!(parse_executable_filters(b"").is_empty());
    }

    #[test]
    fn detects_every_form_of_configuration_include() {
        assert!(has_config_includes(
            b"core.bare\nfalse\0include.path\n/x.cfg\0"
        ));
        assert!(has_config_includes(b"INCLUDE.PATH\n/x.cfg\0"));
        assert!(has_config_includes(
            b"includeif.gitdir:/a/.path\n/x.cfg\0includeif.onbranch:main.path\ny\0"
        ));
        assert!(!has_config_includes(
            b"core.bare\nfalse\0user.name\nincludeif.path\0"
        ));
        assert!(!has_config_includes(b""));
    }

    #[test]
    fn finds_gitlinks_in_the_staged_listing_even_with_odd_names() {
        let listing = b"100644 aaaa 0\tREADME.md\x00160000 bbbb 0\tnested repo\xc3\xb1\0\
            100644 cccc 0\t160000 fake\x00160000 dddd 0\tsub/dir\0";

        assert_eq!(parse_gitlinks(listing), ["nested repoñ", "sub/dir"]);
    }
}
