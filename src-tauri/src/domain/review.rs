use crate::domain::file_copy::looks_secret;
use serde::{Deserialize, Serialize};

pub const MAX_ENTRIES: usize = 2_000;
pub const MAX_DIFF_LINES: usize = 2_000;
pub const MAX_LINE_CHARS: usize = 2_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeCode {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Committed,
    Changed,
    Conflicted,
    Untracked,
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewEntry {
    pub kind: EntryKind,
    pub path: Vec<u8>,
    pub original_path: Option<Vec<u8>>,
    pub staged: Option<ChangeCode>,
    pub unstaged: Option<ChangeCode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewStatus {
    pub entries: Vec<ReviewEntry>,
    pub complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffSource {
    Unstaged,
    Staged,
    Branch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftState {
    UpToDate,
    Moved,
    Rewritten,
    Unavailable,
}

pub fn classify_drift(
    base_commit: &str,
    current: Option<&str>,
    base_is_ancestor: Option<bool>,
) -> DriftState {
    match (current, base_is_ancestor) {
        (None, _) => DriftState::Unavailable,
        (Some(current), _) if current == base_commit => DriftState::UpToDate,
        (Some(_), Some(true)) => DriftState::Moved,
        (Some(_), Some(false)) => DriftState::Rewritten,
        (Some(_), None) => DriftState::Unavailable,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Drift {
    pub state: DriftState,
    pub base_ref: String,
    pub current_commit: Option<String>,
}

pub fn protected_path(path: &[u8]) -> bool {
    String::from_utf8_lossy(path)
        .split('/')
        .filter(|component| !component.is_empty())
        .any(looks_secret)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EntryView {
    pub id: u32,
    pub kind: EntryKind,
    pub path: String,
    pub original_path: Option<String>,
    pub lossy_path: bool,
    pub staged: Option<ChangeCode>,
    pub unstaged: Option<ChangeCode>,
    pub sources: Vec<DiffSource>,
    pub protected: bool,
    pub folder: bool,
    pub added: Option<u64>,
    pub deleted: Option<u64>,
    pub binary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnvironmentReview {
    pub review_id: String,
    pub branch: String,
    pub base_commit_sha: String,
    pub head_commit_sha: Option<String>,
    pub commits_ahead: u32,
    pub drift: Drift,
    pub operation_in_progress: bool,
    pub entries: Vec<EntryView>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FolderListing {
    pub entries: Vec<EntryView>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DiffRequest {
    pub review_id: String,
    pub entry_id: u32,
    pub source: DiffSource,
}

pub fn display_path(path: &[u8]) -> (String, bool) {
    match std::str::from_utf8(path) {
        Ok(text) => (text.to_string(), false),
        Err(_) => (String::from_utf8_lossy(path).into_owned(), true),
    }
}

pub fn immediate_children(folder: &[u8], entries: Vec<ReviewEntry>) -> Vec<ReviewEntry> {
    let mut children: Vec<ReviewEntry> = Vec::new();
    for entry in entries {
        let Some(rest) = entry
            .path
            .strip_prefix(folder)
            .filter(|rest| !rest.is_empty())
        else {
            continue;
        };
        let path = match rest.iter().position(|byte| *byte == b'/') {
            Some(slash) => [folder, &rest[..=slash]].concat(),
            None => entry.path.clone(),
        };
        if !children
            .iter()
            .any(|child| child.path == path && child.kind == entry.kind)
        {
            children.push(ReviewEntry {
                path,
                original_path: None,
                ..entry
            });
        }
    }
    children
}

fn change_code(letter: u8) -> Option<Option<ChangeCode>> {
    Some(match letter {
        b'.' => None,
        b'M' => Some(ChangeCode::Modified),
        b'A' => Some(ChangeCode::Added),
        b'D' => Some(ChangeCode::Deleted),
        b'R' => Some(ChangeCode::Renamed),
        b'C' => Some(ChangeCode::Copied),
        b'T' => Some(ChangeCode::TypeChanged),
        _ => return None,
    })
}

fn parts(record: &[u8], count: usize) -> Option<Vec<&[u8]>> {
    let parts: Vec<&[u8]> = record.splitn(count, |byte| *byte == b' ').collect();
    (parts.len() == count && !parts[count - 1].is_empty()).then_some(parts)
}

pub fn parse_review_status(porcelain: &[u8], output_truncated: bool) -> ReviewStatus {
    let mut records: Vec<&[u8]> = porcelain.split(|byte| *byte == 0).collect();
    let mut complete = !output_truncated;
    if records.last().is_some_and(|last| last.is_empty()) {
        records.pop();
    } else if !records.is_empty() {
        records.pop();
        complete = false;
    }
    let mut records = records.into_iter();
    let mut entries = Vec::new();
    while let Some(record) = records.next() {
        if entries.len() == MAX_ENTRIES {
            complete = false;
            break;
        }
        let entry = match record.first() {
            Some(b'1') => parts(record, 9).and_then(|parts| {
                let (staged, unstaged) = status_pair(parts[1])?;
                Some(ReviewEntry {
                    kind: EntryKind::Changed,
                    path: parts[8].to_vec(),
                    original_path: None,
                    staged,
                    unstaged,
                })
            }),
            Some(b'2') => parts(record, 10).and_then(|parts| {
                let (staged, unstaged) = status_pair(parts[1])?;
                let original = records.next().filter(|field| !field.is_empty())?;
                Some(ReviewEntry {
                    kind: EntryKind::Changed,
                    path: parts[9].to_vec(),
                    original_path: Some(original.to_vec()),
                    staged,
                    unstaged,
                })
            }),
            Some(b'u') => parts(record, 11).map(|parts| ReviewEntry {
                kind: EntryKind::Conflicted,
                path: parts[10].to_vec(),
                original_path: None,
                staged: None,
                unstaged: None,
            }),
            Some(marker @ (b'?' | b'!')) => record
                .get(2..)
                .filter(|path| record.get(1) == Some(&b' ') && !path.is_empty())
                .map(|path| ReviewEntry {
                    kind: if *marker == b'?' {
                        EntryKind::Untracked
                    } else {
                        EntryKind::Ignored
                    },
                    path: path.to_vec(),
                    original_path: None,
                    staged: None,
                    unstaged: None,
                }),
            _ => None,
        };
        match entry {
            Some(entry) => entries.push(entry),
            None => complete = false,
        }
    }
    ReviewStatus { entries, complete }
}

fn status_pair(code: &[u8]) -> Option<(Option<ChangeCode>, Option<ChangeCode>)> {
    match code {
        [staged, unstaged] => Some((change_code(*staged)?, change_code(*unstaged)?)),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumstatEntry {
    pub added: Option<u64>,
    pub deleted: Option<u64>,
    pub path: Vec<u8>,
    pub original_path: Option<Vec<u8>>,
}

impl NumstatEntry {
    pub fn binary(&self) -> bool {
        self.added.is_none() && self.deleted.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Numstat {
    pub entries: Vec<NumstatEntry>,
    pub complete: bool,
}

fn count(field: &[u8]) -> Option<Option<u64>> {
    if field == b"-" {
        return Some(None);
    }
    std::str::from_utf8(field).ok()?.parse().ok().map(Some)
}

pub fn parse_numstat(output: &[u8], output_truncated: bool) -> Numstat {
    let mut fields: Vec<&[u8]> = output.split(|byte| *byte == 0).collect();
    let mut complete = !output_truncated;
    if fields.last().is_some_and(|last| last.is_empty()) {
        fields.pop();
    } else if !fields.is_empty() {
        fields.pop();
        complete = false;
    }
    let mut fields = fields.into_iter();
    let mut entries = Vec::new();
    while let Some(field) = fields.next() {
        if entries.len() == MAX_ENTRIES {
            complete = false;
            break;
        }
        let mut columns = field.splitn(3, |byte| *byte == b'\t');
        let parsed = match (columns.next(), columns.next(), columns.next()) {
            (Some(added), Some(deleted), Some(path)) => match (count(added), count(deleted)) {
                (Some(added), Some(deleted)) if added.is_some() == deleted.is_some() => {
                    if path.is_empty() {
                        match (fields.next(), fields.next()) {
                            (Some(old), Some(new)) if !old.is_empty() && !new.is_empty() => {
                                Some((added, deleted, new, Some(old)))
                            }
                            _ => None,
                        }
                    } else {
                        Some((added, deleted, path, None))
                    }
                }
                _ => None,
            },
            _ => None,
        };
        match parsed {
            Some((added, deleted, path, original)) => entries.push(NumstatEntry {
                added,
                deleted,
                path: path.to_vec(),
                original_path: original.map(<[u8]>::to_vec),
            }),
            None => complete = false,
        }
    }
    Numstat { entries, complete }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LineKind {
    Context,
    Added,
    Removed,
    NoNewline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiffLine {
    pub kind: LineKind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileDiff {
    pub binary: bool,
    pub hunks: Vec<Hunk>,
    pub complete: bool,
    pub lossy: bool,
}

pub fn parse_patch(patch: &[u8], output_truncated: bool) -> FileDiff {
    let mut lines: Vec<&[u8]> = patch.split(|byte| *byte == b'\n').collect();
    let mut complete = !output_truncated;
    if output_truncated || lines.last().is_some_and(|last| last.is_empty()) {
        lines.pop();
    }
    let mut diff = FileDiff {
        binary: false,
        hunks: Vec::new(),
        complete: true,
        lossy: false,
    };
    let mut total = 0;
    for line in lines {
        if diff.hunks.is_empty() && !line.starts_with(b"@@ ") {
            if line.starts_with(b"Binary files ") || line.starts_with(b"GIT binary patch") {
                diff.binary = true;
            }
            continue;
        }
        let text = |bytes: &[u8], diff: &mut FileDiff, complete: &mut bool| {
            let (text, lossy) = display_path(bytes);
            diff.lossy |= lossy;
            if text.chars().count() > MAX_LINE_CHARS {
                *complete = false;
                text.chars().take(MAX_LINE_CHARS).collect()
            } else {
                text
            }
        };
        if line.starts_with(b"@@ ") {
            let header = text(line, &mut diff, &mut complete);
            diff.hunks.push(Hunk {
                header,
                lines: Vec::new(),
            });
            continue;
        }
        let kind = match line.first() {
            Some(b' ') => LineKind::Context,
            Some(b'+') => LineKind::Added,
            Some(b'-') => LineKind::Removed,
            Some(b'\\') => LineKind::NoNewline,
            None => LineKind::Context,
            _ => {
                complete = false;
                continue;
            }
        };
        if total == MAX_DIFF_LINES {
            complete = false;
            break;
        }
        total += 1;
        let body = text(line.get(1..).unwrap_or_default(), &mut diff, &mut complete);
        if let Some(hunk) = diff.hunks.last_mut() {
            hunk.lines.push(DiffLine { kind, text: body });
        }
    }
    diff.complete = complete;
    diff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_listing_keeps_only_its_immediate_children() {
        let entry = |kind, path: &str| ReviewEntry {
            kind,
            path: path.as_bytes().to_vec(),
            original_path: None,
            staged: None,
            unstaged: None,
        };
        let listing = vec![
            entry(EntryKind::Untracked, "docs/a.md"),
            entry(EntryKind::Untracked, "docs/deep/b.md"),
            entry(EntryKind::Untracked, "docs/deep/c.md"),
            entry(EntryKind::Ignored, "docs/deep/d.o"),
            entry(EntryKind::Untracked, "docs/z.md"),
            entry(EntryKind::Untracked, "elsewhere/x.md"),
            entry(EntryKind::Untracked, "docs-other/y.md"),
        ];

        let children = immediate_children(b"docs/", listing);

        let summary: Vec<(EntryKind, &[u8])> = children
            .iter()
            .map(|child| (child.kind, child.path.as_slice()))
            .collect();
        assert_eq!(
            summary,
            [
                (EntryKind::Untracked, &b"docs/a.md"[..]),
                (EntryKind::Untracked, b"docs/deep/"),
                (EntryKind::Ignored, b"docs/deep/"),
                (EntryKind::Untracked, b"docs/z.md"),
            ]
        );
    }

    #[test]
    fn drift_is_classified_from_the_base_commit_and_the_current_branch() {
        use DriftState::*;
        assert_eq!(classify_drift("a", Some("a"), None), UpToDate);
        assert_eq!(classify_drift("a", Some("b"), Some(true)), Moved);
        assert_eq!(classify_drift("a", Some("b"), Some(false)), Rewritten);
        assert_eq!(classify_drift("a", Some("b"), None), Unavailable);
        assert_eq!(classify_drift("a", None, None), Unavailable);
        assert_eq!(classify_drift("a", None, Some(true)), Unavailable);
    }

    #[test]
    fn paths_with_a_secret_looking_component_are_protected() {
        for path in [
            ".env",
            "config/.env.local",
            "keys/server.pem",
            "secrets/readme.txt",
            "a/credentials.json",
        ] {
            assert!(protected_path(path.as_bytes()), "{path}");
        }
        for path in ["README.md", "src/main.rs", "docs/"] {
            assert!(!protected_path(path.as_bytes()), "{path}");
        }
    }

    #[test]
    fn reads_every_kind_of_entry_keeping_staged_and_unstaged_apart() {
        let porcelain = b"1 M. N... 100644 100644 100644 aa bb staged.txt\0\
            1 .M N... 100644 100644 100644 aa bb unstaged.txt\0\
            1 MM N... 100644 100644 100644 aa bb both.txt\0\
            1 A. N... 000000 100644 100644 00 bb new file.txt\0\
            1 .D N... 100644 100644 000000 aa bb gone.txt\0\
            2 R. N... 100644 100644 100644 aa bb R100 renamed to.txt\0renamed from.txt\0\
            u UU N... 100644 100644 100644 100644 aa bb cc conflict.txt\0\
            ? untracked dir/\0\
            ! target/\0";

        let status = parse_review_status(porcelain, false);

        assert!(status.complete);
        let summary: Vec<_> = status
            .entries
            .iter()
            .map(|entry| {
                (
                    display_path(&entry.path).0,
                    entry.kind,
                    entry.staged,
                    entry.unstaged,
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                (
                    "staged.txt".to_string(),
                    EntryKind::Changed,
                    Some(ChangeCode::Modified),
                    None
                ),
                (
                    "unstaged.txt".to_string(),
                    EntryKind::Changed,
                    None,
                    Some(ChangeCode::Modified)
                ),
                (
                    "both.txt".to_string(),
                    EntryKind::Changed,
                    Some(ChangeCode::Modified),
                    Some(ChangeCode::Modified)
                ),
                (
                    "new file.txt".to_string(),
                    EntryKind::Changed,
                    Some(ChangeCode::Added),
                    None
                ),
                (
                    "gone.txt".to_string(),
                    EntryKind::Changed,
                    None,
                    Some(ChangeCode::Deleted)
                ),
                (
                    "renamed to.txt".to_string(),
                    EntryKind::Changed,
                    Some(ChangeCode::Renamed),
                    None
                ),
                (
                    "conflict.txt".to_string(),
                    EntryKind::Conflicted,
                    None,
                    None
                ),
                (
                    "untracked dir/".to_string(),
                    EntryKind::Untracked,
                    None,
                    None
                ),
                ("target/".to_string(), EntryKind::Ignored, None, None),
            ]
        );
        assert_eq!(
            status.entries[5].original_path.as_deref(),
            Some(&b"renamed from.txt"[..])
        );
    }

    #[test]
    fn keeps_the_exact_bytes_of_paths_that_are_not_utf8_or_hold_newlines() {
        let mut porcelain = b"? caf".to_vec();
        porcelain.extend([0xe9, b'\n', b'x', 0]);

        let status = parse_review_status(&porcelain, false);

        assert_eq!(
            status.entries[0].path,
            [b'c', b'a', b'f', 0xe9, b'\n', b'x']
        );
        let (text, lossy) = display_path(&status.entries[0].path);
        assert!(lossy && text.contains('\u{fffd}'));
        assert_eq!(
            display_path("ñandú/✓.txt".as_bytes()),
            ("ñandú/✓.txt".to_string(), false)
        );
    }

    #[test]
    fn a_cut_or_malformed_listing_is_never_complete() {
        let cut = parse_review_status(b"? one.txt\0? tw", true);
        let unterminated = parse_review_status(b"? one.txt\0? tw", false);
        let rename_without_origin = parse_review_status(
            b"2 R. N... 100644 100644 100644 aa bb R100 new.txt\0",
            false,
        );
        let malformed = parse_review_status(b"? one.txt\0x nonsense\0? \0Z\0", false);

        assert_eq!(cut.entries.len(), 1);
        assert!(!cut.complete && !unterminated.complete);
        assert!(!rename_without_origin.complete && rename_without_origin.entries.is_empty());
        assert_eq!(malformed.entries.len(), 1);
        assert!(!malformed.complete);
        assert!(parse_review_status(b"", false).complete);
        assert!(!parse_review_status(b"", true).complete);
    }

    #[test]
    fn stops_at_the_entry_limit_and_says_so() {
        let mut porcelain = Vec::new();
        for index in 0..=MAX_ENTRIES {
            porcelain.extend(format!("? f{index}").bytes());
            porcelain.push(0);
        }

        let status = parse_review_status(&porcelain, false);

        assert_eq!(status.entries.len(), MAX_ENTRIES);
        assert!(!status.complete);
    }

    #[test]
    fn reads_counts_binary_files_and_renames_from_numstat() {
        let output = b"3\t1\tsrc/a b.rs\0-\t-\timage.png\x002\t0\t\0old name.txt\0new name.txt\0";

        let numstat = parse_numstat(output, false);

        assert!(numstat.complete);
        assert_eq!(numstat.entries[0].added, Some(3));
        assert_eq!(numstat.entries[0].path, b"src/a b.rs");
        assert!(numstat.entries[1].binary());
        assert_eq!(numstat.entries[2].path, b"new name.txt");
        assert_eq!(
            numstat.entries[2].original_path.as_deref(),
            Some(&b"old name.txt"[..])
        );
    }

    #[test]
    fn malformed_or_cut_numstat_is_never_complete() {
        assert!(!parse_numstat(b"1\t1\ta\0x\ty\tb\0", false).complete);
        assert!(!parse_numstat(b"1\t-\tmixed\0", false).complete);
        assert!(!parse_numstat(b"1\t1\ta\0", true).complete);
        assert!(!parse_numstat(b"2\t0\t\0only-one\0", false).complete);
        assert!(parse_numstat(b"", false).complete);
    }

    #[test]
    fn reads_hunks_with_context_changes_and_the_missing_newline_marker() {
        let patch = b"diff --git a/a.txt b/a.txt\nindex 1..2 100644\n--- a/a.txt\n+++ b/a.txt\n\
            @@ -1,3 +1,3 @@ fn main\n keep\n-old\n+new\r\n\\ No newline at end of file\n\
            @@ -9 +9 @@\n-x\n+y\n";

        let diff = parse_patch(patch, false);

        assert!(diff.complete && !diff.binary && !diff.lossy);
        assert_eq!(diff.hunks.len(), 2);
        assert_eq!(diff.hunks[0].header, "@@ -1,3 +1,3 @@ fn main");
        let kinds: Vec<_> = diff.hunks[0].lines.iter().map(|line| line.kind).collect();
        assert_eq!(
            kinds,
            [
                LineKind::Context,
                LineKind::Removed,
                LineKind::Added,
                LineKind::NoNewline
            ]
        );
        assert_eq!(diff.hunks[0].lines[2].text, "new\r");
    }

    #[test]
    fn binary_patches_have_no_hunks_and_are_flagged() {
        let patch = b"diff --git a/i.png b/i.png\nindex 1..2 100644\nBinary files a/i.png and b/i.png differ\n";

        let diff = parse_patch(patch, false);

        assert!(diff.binary && diff.hunks.is_empty() && diff.complete);
    }

    #[test]
    fn a_patch_beyond_the_limits_is_cut_and_marked_partial() {
        let mut long = b"@@ -1 +1 @@\n".to_vec();
        for _ in 0..=MAX_DIFF_LINES {
            long.extend(b"+line\n");
        }
        let wide = format!("@@ -1 +1 @@\n+{}\n", "x".repeat(MAX_LINE_CHARS + 50));

        let many = parse_patch(&long, false);
        let wide = parse_patch(wide.as_bytes(), false);
        let cut = parse_patch(b"@@ -1 +1 @@\n+one\n+tw", true);

        assert_eq!(many.hunks[0].lines.len(), MAX_DIFF_LINES);
        assert!(!many.complete && !wide.complete && !cut.complete);
        assert_eq!(wide.hunks[0].lines[0].text.chars().count(), MAX_LINE_CHARS);
        assert_eq!(cut.hunks[0].lines.len(), 1);
    }

    #[test]
    fn invalid_utf8_in_a_patch_is_replaced_and_flagged() {
        let mut patch = b"@@ -1 +1 @@\n+caf".to_vec();
        patch.extend([0xe9, b'\n']);

        let diff = parse_patch(&patch, false);

        assert!(diff.lossy && diff.complete);
        assert!(diff.hunks[0].lines[0].text.contains('\u{fffd}'));
    }
}
