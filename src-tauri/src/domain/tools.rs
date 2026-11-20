use crate::domain::agent_run::ErrorKind;
use crate::domain::path_jail::{self, PathError};
use crate::domain::review::{ChangeCode, DiffSource, EntryKind, ReviewEntry, protected_path};
use serde_json::{Value, json};
use std::path::PathBuf;

pub const MAX_ITERATIONS: u32 = 20;
pub const MAX_CALLS: u32 = 40;
pub const MAX_OUTPUT_BYTES: usize = 128 * 1024;
pub const MAX_ARGUMENT_BYTES: usize = 4096;
pub const MAX_LIST_BYTES: usize = 8 * 1024;
pub const MAX_LIST_ENTRIES: usize = 2_000;
pub const MAX_READ_BYTES: usize = 16 * 1024;
pub const MAX_DIFF_BYTES: usize = 32 * 1024;
pub const MAX_SEARCH_BYTES: usize = 8 * 1024;
pub const MAX_SEARCH_FILES: u32 = 2_000;
pub const MAX_SEARCH_DIRS: u32 = 1_000;
pub const MAX_SEARCH_DEPTH: usize = 16;
pub const MAX_SEARCH_FILE_BYTES: usize = 256 * 1024;
pub const MAX_SEARCH_TOTAL_BYTES: usize = 8 * 1024 * 1024;
const MAX_QUERY_BYTES: usize = 256;
const MAX_MATCH_CHARS: usize = 160;
const READ_NOTE_RESERVE: usize = 96;
pub const READ_WINDOW: usize = MAX_READ_BYTES - READ_NOTE_RESERVE;
const TRUNCATION_NOTE: &str = "[truncated]\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolContext {
    pub workspace_id: String,
    pub task_id: String,
    pub worktree_id: String,
    pub root: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListedKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedEntry {
    pub name: String,
    pub kind: ListedKind,
}

pub struct Definition {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Value,
}

pub fn definitions() -> Vec<Definition> {
    vec![
        Definition {
            name: "list_dir",
            description: "List the entries of a directory of the task working directory. Directories end with '/', symbolic links with '@'. Read only.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory relative to the working directory root. Defaults to the root."
                    }
                },
                "additionalProperties": false
            }),
        },
        Definition {
            name: "read_file",
            description: "Read a UTF-8 text file of the task working directory, up to about 16 KiB per call. When the file is longer the result ends with the offset to continue from. Binary files and files that look like secrets are refused. Read only.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File relative to the working directory root."
                    },
                    "offset": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Byte offset to start from, as given by a previous truncated read. Defaults to 0."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
        Definition {
            name: "search",
            description: "Search the text files of the task working directory for a literal string. Returns up to about 8 KiB of 'path:line: text' matches. Large trees are only partially searched. Read only.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Literal text to look for (no patterns), at most 256 bytes."
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory to search, relative to the working directory root. Defaults to the root."
                    },
                    "ignore_case": {
                        "type": "boolean",
                        "description": "Match regardless of letter case. Defaults to false."
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        },
        Definition {
            name: "git_status",
            description: "Show the files changed in the task working directory (staged, unstaged, conflicted and untracked). Read only.",
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Definition {
            name: "git_diff",
            description: "Show the Git diff of one file of the task working directory, up to about 32 KiB. 'unstaged' compares the working tree with the index, 'staged' the index with HEAD, 'branch' the task branch with the commit it started from. Read only.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File relative to the working directory root."
                    },
                    "source": {
                        "type": "string",
                        "enum": ["unstaged", "staged", "branch"],
                        "description": "Which changes to show. Defaults to unstaged."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
    ]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    ListDir {
        path: String,
    },
    ReadFile {
        path: String,
        offset: u64,
    },
    Search {
        query: String,
        path: String,
        ignore_case: bool,
    },
    GitStatus,
    GitDiff {
        source: DiffSource,
        path: String,
    },
}

impl Request {
    pub fn is_git(&self) -> bool {
        matches!(self, Request::GitStatus | Request::GitDiff { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prepared {
    Rejected {
        named: bool,
    },
    Audited {
        arguments: String,
        request: Result<Request, &'static str>,
    },
}

pub fn prepare(name: &str, raw_arguments: &str, oversized: bool) -> Prepared {
    let well_formed_name = !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'_');
    let rejected = Prepared::Rejected {
        named: well_formed_name,
    };
    if oversized || raw_arguments.len() > MAX_ARGUMENT_BYTES || !well_formed_name {
        return rejected;
    }
    let trimmed = raw_arguments.trim();
    let value: Value = if trimmed.is_empty() {
        json!({})
    } else {
        match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(_) => return rejected,
        }
    };
    if !value.is_object() {
        return rejected;
    }
    let arguments = value.to_string();
    if arguments.len() > MAX_ARGUMENT_BYTES {
        return rejected;
    }
    let request = match name {
        "list_dir" => list_dir_request(&value),
        "read_file" => read_file_request(&value),
        "search" => search_request(&value),
        "git_status" => fields(&value, &[]).map(|_| Request::GitStatus),
        "git_diff" => git_diff_request(&value),
        _ => Err("unknown_tool"),
    };
    Prepared::Audited { arguments, request }
}

fn fields<'a>(
    arguments: &'a Value,
    allowed: &[&str],
) -> Result<&'a serde_json::Map<String, Value>, &'static str> {
    let fields = arguments.as_object().ok_or("invalid_arguments")?;
    if fields.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err("invalid_arguments");
    }
    Ok(fields)
}

fn optional_path(fields: &serde_json::Map<String, Value>) -> Result<String, &'static str> {
    let path = match fields.get("path") {
        None => "",
        Some(Value::String(path)) => path.as_str(),
        Some(_) => return Err("invalid_arguments"),
    };
    path_jail::parse(path).map_err(PathError::code)
}

fn list_dir_request(arguments: &Value) -> Result<Request, &'static str> {
    let fields = fields(arguments, &["path"])?;
    Ok(Request::ListDir {
        path: optional_path(fields)?,
    })
}

fn read_file_request(arguments: &Value) -> Result<Request, &'static str> {
    let fields = fields(arguments, &["path", "offset"])?;
    if !fields.contains_key("path") {
        return Err("invalid_arguments");
    }
    let path = optional_path(fields)?;
    if path.is_empty() {
        return Err(PathError::Invalid.code());
    }
    let offset = match fields.get("offset") {
        None => 0,
        Some(value) => value.as_u64().ok_or("invalid_arguments")?,
    };
    Ok(Request::ReadFile { path, offset })
}

fn search_request(arguments: &Value) -> Result<Request, &'static str> {
    let fields = fields(arguments, &["query", "path", "ignore_case"])?;
    let query = match fields.get("query") {
        Some(Value::String(query))
            if !query.is_empty()
                && query.len() <= MAX_QUERY_BYTES
                && !query.chars().any(char::is_control) =>
        {
            query.clone()
        }
        _ => return Err("invalid_arguments"),
    };
    let ignore_case = match fields.get("ignore_case") {
        None => false,
        Some(Value::Bool(value)) => *value,
        Some(_) => return Err("invalid_arguments"),
    };
    Ok(Request::Search {
        query,
        path: optional_path(fields)?,
        ignore_case,
    })
}

fn git_diff_request(arguments: &Value) -> Result<Request, &'static str> {
    let fields = fields(arguments, &["path", "source"])?;
    if !fields.contains_key("path") {
        return Err("invalid_arguments");
    }
    let path = optional_path(fields)?;
    if path.is_empty() {
        return Err(PathError::Invalid.code());
    }
    let source = match fields.get("source") {
        None => DiffSource::Unstaged,
        Some(Value::String(source)) => match source.as_str() {
            "unstaged" => DiffSource::Unstaged,
            "staged" => DiffSource::Staged,
            "branch" => DiffSource::Branch,
            _ => return Err("invalid_arguments"),
        },
        Some(_) => return Err("invalid_arguments"),
    };
    Ok(Request::GitDiff { source, path })
}

#[derive(Debug, PartialEq, Eq)]
pub struct Window {
    pub text: String,
    pub next_offset: Option<u64>,
}

pub fn decode_window(chunk: &[u8], offset: u64, more_after: bool) -> Result<Window, &'static str> {
    if offset > 0 && chunk.first().is_some_and(|byte| byte & 0xC0 == 0x80) {
        return Err("invalid_offset");
    }
    if chunk.contains(&0) {
        return Err("binary_file");
    }
    let valid = match std::str::from_utf8(chunk) {
        Ok(text) => text.len(),
        Err(error) if error.error_len().is_none() && more_after => error.valid_up_to(),
        Err(_) => return Err("binary_file"),
    };
    let text = String::from_utf8_lossy(&chunk[..valid]).into_owned();
    let more = more_after || valid < chunk.len();
    Ok(Window {
        text,
        next_offset: more.then(|| offset + valid as u64),
    })
}

pub fn render_window(window: Window) -> (String, bool) {
    let mut text = window.text;
    let truncated = window.next_offset.is_some();
    if let Some(next) = window.next_offset {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&format!("[truncated: continue with offset {next}]\n"));
    }
    (text, truncated)
}

pub struct Needle {
    text: String,
    ignore_case: bool,
}

impl Needle {
    pub fn new(query: &str, ignore_case: bool) -> Needle {
        Needle {
            text: if ignore_case {
                query.to_lowercase()
            } else {
                query.to_string()
            },
            ignore_case,
        }
    }

    fn found_in(&self, line: &str) -> bool {
        if self.ignore_case {
            line.to_lowercase().contains(&self.text)
        } else {
            line.contains(&self.text)
        }
    }
}

#[derive(Default)]
pub struct SearchReport {
    text: String,
    full: bool,
}

fn shown(text: &str) -> String {
    text.trim()
        .chars()
        .take(MAX_MATCH_CHARS)
        .map(|character| {
            if character.is_control() {
                '?'
            } else {
                character
            }
        })
        .collect()
}

impl SearchReport {
    pub fn is_full(&self) -> bool {
        self.full
    }

    pub fn add_file(&mut self, path: &str, content: &str, needle: &Needle) {
        let path = shown(path);
        for (index, line) in content.lines().enumerate() {
            if !needle.found_in(line) {
                continue;
            }
            let entry = format!("{path}:{}: {}\n", index + 1, shown(line));
            if self.text.len() + entry.len() + TRUNCATION_NOTE.len() > MAX_SEARCH_BYTES {
                self.full = true;
                return;
            }
            self.text.push_str(&entry);
        }
    }

    pub fn finish(mut self, incomplete: bool) -> (String, bool) {
        let truncated = self.full || incomplete;
        if self.text.is_empty() && !truncated {
            self.text.push_str("no matches\n");
        }
        if truncated {
            self.text.push_str(TRUNCATION_NOTE);
        }
        (self.text, truncated)
    }
}

pub fn render_listing(entries: &[ListedEntry], cut_by_source: bool) -> (String, bool) {
    let mut lines: Vec<String> = entries
        .iter()
        .filter(|entry| !path_jail::is_hidden_entry(&entry.name))
        .map(|entry| {
            let marker = match entry.kind {
                ListedKind::Directory => "/",
                ListedKind::Symlink => "@",
                ListedKind::File | ListedKind::Other => "",
            };
            format!("{}{marker}", entry.name.replace(['\n', '\r'], "?"))
        })
        .collect();
    lines.sort();
    let mut text = String::new();
    let mut truncated = cut_by_source;
    for line in lines {
        if text.len() + line.len() + 1 + TRUNCATION_NOTE.len() > MAX_LIST_BYTES {
            truncated = true;
            break;
        }
        text.push_str(&line);
        text.push('\n');
    }
    if truncated {
        text.push_str(TRUNCATION_NOTE);
    }
    (text, truncated)
}

fn letter(code: Option<ChangeCode>) -> char {
    match code {
        None => '.',
        Some(ChangeCode::Added) => 'A',
        Some(ChangeCode::Modified) => 'M',
        Some(ChangeCode::Deleted) => 'D',
        Some(ChangeCode::Renamed) => 'R',
        Some(ChangeCode::Copied) => 'C',
        Some(ChangeCode::TypeChanged) => 'T',
    }
}

fn plain(path: &[u8]) -> String {
    String::from_utf8_lossy(path)
        .chars()
        .map(|character| {
            if character.is_control() {
                '?'
            } else {
                character
            }
        })
        .collect()
}

pub fn render_status(entries: &[ReviewEntry], complete: bool) -> (String, bool) {
    let mut hidden = 0usize;
    let mut lines = Vec::new();
    for entry in entries {
        if entry.kind == EntryKind::Ignored {
            continue;
        }
        if protected_path(&entry.path) || entry.original_path.as_deref().is_some_and(protected_path)
        {
            hidden += 1;
            continue;
        }
        let code = match entry.kind {
            EntryKind::Untracked => "??".to_string(),
            EntryKind::Conflicted => "UU".to_string(),
            _ => format!("{}{}", letter(entry.staged), letter(entry.unstaged)),
        };
        let path = plain(&entry.path);
        lines.push(match &entry.original_path {
            Some(original) => format!("{code} {} -> {path}", plain(original)),
            None => format!("{code} {path}"),
        });
    }
    let mut text = String::new();
    let mut truncated = !complete;
    for line in lines {
        if text.len() + line.len() + 1 + TRUNCATION_NOTE.len() > MAX_LIST_BYTES {
            truncated = true;
            break;
        }
        text.push_str(&line);
        text.push('\n');
    }
    if hidden > 0 {
        text.push_str(&format!("[{hidden} protected entries hidden]\n"));
    }
    if text.is_empty() {
        text.push_str("working tree clean\n");
    }
    if truncated {
        text.push_str(TRUNCATION_NOTE);
    }
    (text, truncated)
}

pub fn render_diff(patch: &[u8], cut_by_source: bool) -> (String, bool) {
    let text = String::from_utf8_lossy(patch).replace('\0', "");
    let limit = MAX_DIFF_BYTES - TRUNCATION_NOTE.len();
    let mut truncated = cut_by_source;
    let mut shown = String::new();
    for line in text.split_inclusive('\n') {
        if shown.len() + line.len() > limit {
            truncated = true;
            break;
        }
        shown.push_str(line);
    }
    if shown.is_empty() && !truncated {
        return ("no differences\n".into(), false);
    }
    if !shown.is_empty() && !shown.ends_with('\n') {
        shown.push('\n');
    }
    if truncated {
        shown.push_str(TRUNCATION_NOTE);
    }
    (shown, truncated)
}

#[derive(Debug, Default)]
pub struct Budget {
    iterations: u32,
    calls: u32,
    output: usize,
}

impl Budget {
    pub fn start_iteration(&mut self) -> Result<(), ErrorKind> {
        if self.iterations >= MAX_ITERATIONS {
            return Err(ErrorKind::IterationBudgetExhausted);
        }
        self.iterations += 1;
        Ok(())
    }

    pub fn iteration(&self) -> u32 {
        self.iterations
    }

    pub fn admit_calls(&mut self, count: usize) -> Result<(), ErrorKind> {
        let total = u32::try_from(count)
            .ok()
            .and_then(|count| self.calls.checked_add(count))
            .filter(|total| *total <= MAX_CALLS)
            .ok_or(ErrorKind::ToolCallBudgetExhausted)?;
        self.calls = total;
        Ok(())
    }

    pub fn add_output(&mut self, bytes: usize) -> Result<(), ErrorKind> {
        let total = self
            .output
            .checked_add(bytes)
            .filter(|total| *total <= MAX_OUTPUT_BYTES)
            .ok_or(ErrorKind::ToolOutputBudgetExhausted)?;
        self.output = total;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, kind: ListedKind) -> ListedEntry {
        ListedEntry {
            name: name.into(),
            kind,
        }
    }

    #[test]
    fn a_missing_path_lists_the_root() {
        assert_eq!(
            prepare("list_dir", "", false),
            Prepared::Audited {
                arguments: "{}".into(),
                request: Ok(Request::ListDir { path: "".into() })
            }
        );
        assert_eq!(
            prepare("list_dir", r#"{"path":"./src/"}"#, false),
            Prepared::Audited {
                arguments: r#"{"path":"./src/"}"#.into(),
                request: Ok(Request::ListDir { path: "src".into() })
            }
        );
    }

    #[test]
    fn calls_whose_arguments_cannot_be_stored_are_rejected_and_keep_only_a_valid_name() {
        for (name, arguments, oversized, named) in [
            ("list_dir", "{not json", false, true),
            ("list_dir", "[]", false, true),
            ("list_dir", "\"x\"", false, true),
            ("list_dir", "{}", true, true),
            ("List-Dir", "{}", false, false),
            ("", "{}", false, false),
        ] {
            assert_eq!(
                prepare(name, arguments, oversized),
                Prepared::Rejected { named }
            );
        }
        let large = format!(r#"{{"path":"{}"}}"#, "a".repeat(MAX_ARGUMENT_BYTES));
        assert_eq!(
            prepare("list_dir", &large, false),
            Prepared::Rejected { named: true }
        );
    }

    #[test]
    fn invalid_but_faithful_calls_are_audited_with_their_reason() {
        let reason = |name: &str, arguments: &str| match prepare(name, arguments, false) {
            Prepared::Audited { request, .. } => request.unwrap_err(),
            Prepared::Rejected { .. } => "rejected",
        };
        assert_eq!(reason("run_command", "{}"), "unknown_tool");
        assert_eq!(reason("list_dir", r#"{"path":3}"#), "invalid_arguments");
        assert_eq!(reason("list_dir", r#"{"other":1}"#), "invalid_arguments");
        assert_eq!(reason("list_dir", r#"{"path":"../x"}"#), "path_outside");
        assert_eq!(reason("list_dir", r#"{"path":".git"}"#), "path_reserved");
        assert_eq!(reason("list_dir", r#"{"path":"a\\b"}"#), "path_invalid");
        assert_eq!(reason("list_dir", r#"{"path":".env"}"#), "path_protected");
    }

    #[test]
    fn listings_mark_kinds_hide_internals_and_stay_sorted() {
        let (text, truncated) = render_listing(
            &[
                entry("b.txt", ListedKind::File),
                entry(".env", ListedKind::File),
                entry(".git", ListedKind::File),
                entry("a", ListedKind::Directory),
                entry("link", ListedKind::Symlink),
                entry("we\nird", ListedKind::File),
            ],
            false,
        );

        assert_eq!(text, "a/\nb.txt\nlink@\nwe?ird\n");
        assert!(!truncated);
    }

    #[test]
    fn listings_are_cut_at_the_byte_limit_on_a_whole_line() {
        let entries: Vec<_> = (0..2_000)
            .map(|index| entry(&format!("file-{index:05}"), ListedKind::File))
            .collect();

        let (text, truncated) = render_listing(&entries, false);

        assert!(truncated);
        assert!(text.len() <= MAX_LIST_BYTES);
        assert!(text.ends_with("[truncated]\n"));
        assert!(render_listing(&entries[..3], true).1);
    }

    #[test]
    fn the_budget_enforces_every_limit_exactly() {
        let mut budget = Budget::default();
        for _ in 0..MAX_ITERATIONS {
            assert!(budget.start_iteration().is_ok());
        }
        assert_eq!(
            budget.start_iteration(),
            Err(ErrorKind::IterationBudgetExhausted)
        );

        assert!(budget.admit_calls(39).is_ok());
        assert!(budget.admit_calls(1).is_ok());
        assert_eq!(
            budget.admit_calls(1),
            Err(ErrorKind::ToolCallBudgetExhausted)
        );

        assert!(budget.add_output(MAX_OUTPUT_BYTES).is_ok());
        assert_eq!(
            budget.add_output(1),
            Err(ErrorKind::ToolOutputBudgetExhausted)
        );
    }

    #[test]
    fn a_batch_larger_than_the_remaining_calls_is_refused_whole() {
        let mut budget = Budget::default();
        assert!(budget.admit_calls(38).is_ok());

        assert_eq!(
            budget.admit_calls(3),
            Err(ErrorKind::ToolCallBudgetExhausted)
        );
        assert!(budget.admit_calls(2).is_ok());
    }

    fn request(name: &str, arguments: &str) -> Result<Request, &'static str> {
        match prepare(name, arguments, false) {
            Prepared::Audited { request, .. } => request,
            Prepared::Rejected { .. } => Err("rejected"),
        }
    }

    #[test]
    fn read_file_needs_a_path_inside_the_jail_and_an_unsigned_integer_offset() {
        assert_eq!(
            request("read_file", r#"{"path":"src/a.rs","offset":10}"#),
            Ok(Request::ReadFile {
                path: "src/a.rs".into(),
                offset: 10
            })
        );
        assert_eq!(
            request("read_file", r#"{"path":"a"}"#),
            Ok(Request::ReadFile {
                path: "a".into(),
                offset: 0
            })
        );
        for (arguments, reason) in [
            ("{}", "invalid_arguments"),
            (r#"{"path":""}"#, "path_invalid"),
            (r#"{"path":"."}"#, "path_invalid"),
            (r#"{"path":"a","offset":-1}"#, "invalid_arguments"),
            (r#"{"path":"a","offset":1.5}"#, "invalid_arguments"),
            (r#"{"path":"a","offset":"1"}"#, "invalid_arguments"),
            (r#"{"path":"a","more":1}"#, "invalid_arguments"),
            (r#"{"path":"../a"}"#, "path_outside"),
            (r#"{"path":"/etc/passwd"}"#, "path_outside"),
            (r#"{"path":".env"}"#, "path_protected"),
            (r#"{"path":"a/.git/config"}"#, "path_reserved"),
        ] {
            assert_eq!(request("read_file", arguments), Err(reason), "{arguments}");
        }
    }

    #[test]
    fn search_takes_a_literal_query_of_bounded_size() {
        assert_eq!(
            request(
                "search",
                r#"{"query":"TODO","path":"src","ignore_case":true}"#
            ),
            Ok(Request::Search {
                query: "TODO".into(),
                path: "src".into(),
                ignore_case: true
            })
        );
        let long = format!(r#"{{"query":"{}"}}"#, "a".repeat(257));
        for arguments in [
            "{}",
            r#"{"query":""}"#,
            r#"{"query":3}"#,
            r#"{"query":"a\nb"}"#,
            r#"{"query":"a","ignore_case":"yes"}"#,
            r#"{"query":"a","extra":1}"#,
            long.as_str(),
        ] {
            assert_eq!(
                request("search", arguments),
                Err("invalid_arguments"),
                "{arguments}"
            );
        }
        assert_eq!(
            request("search", r#"{"query":"a","path":".env"}"#),
            Err("path_protected")
        );
    }

    #[test]
    fn a_window_never_splits_a_character_and_points_to_the_next_offset() {
        let text = "aé".as_bytes();

        let window = decode_window(&text[..2], 0, true).unwrap();

        assert_eq!(window.text, "a");
        assert_eq!(window.next_offset, Some(1));
        let window = decode_window(&text[1..], 1, false).unwrap();
        assert_eq!((window.text.as_str(), window.next_offset), ("é", None));
        assert_eq!(decode_window(&text[2..], 2, false), Err("invalid_offset"));
        assert_eq!(decode_window(&text[..2], 0, false), Err("binary_file"));
        assert_eq!(decode_window(b"a\0b", 0, false), Err("binary_file"));
        assert_eq!(decode_window(&[0xff, 0xfe], 0, true), Err("binary_file"));
    }

    #[test]
    fn a_truncated_window_ends_with_the_offset_to_continue_and_fits_the_reserve() {
        let (text, truncated) = render_window(Window {
            text: "abc".into(),
            next_offset: Some(u64::MAX),
        });

        assert!(truncated);
        assert_eq!(
            text,
            format!("abc\n[truncated: continue with offset {}]\n", u64::MAX)
        );
        assert!(text.len() - 3 <= READ_NOTE_RESERVE);
        assert_eq!(
            render_window(Window {
                text: "abc".into(),
                next_offset: None
            }),
            ("abc".into(), false)
        );
    }

    #[test]
    fn search_reports_numbered_shortened_clean_lines_and_never_exceeds_its_cap() {
        let needle = Needle::new("key", true);
        let mut report = SearchReport::default();
        let long = format!("KEY {}", "x".repeat(500));

        report.add_file("a\tb.txt", "nothing\n\u{1b}[31mKey\n", &needle);
        report.add_file("c.txt", &long, &needle);
        for index in 0..1000 {
            report.add_file(&format!("many/{index}.txt"), "key\n", &needle);
        }

        assert!(report.is_full());
        let (text, truncated) = report.finish(false);
        assert!(truncated && text.len() <= MAX_SEARCH_BYTES);
        assert!(text.starts_with("a?b.txt:2: ?[31mKey\n"));
        assert!(text.lines().nth(1).unwrap().len() < 200);
        assert!(!text.contains('\u{1b}'));
        assert_eq!(
            SearchReport::default().finish(false),
            ("no matches\n".into(), false)
        );
        assert_eq!(
            SearchReport::default().finish(true),
            ("[truncated]\n".into(), true)
        );
    }

    fn status_entry(
        kind: EntryKind,
        path: &str,
        staged: Option<ChangeCode>,
        unstaged: Option<ChangeCode>,
    ) -> ReviewEntry {
        ReviewEntry {
            kind,
            path: path.as_bytes().to_vec(),
            original_path: None,
            staged,
            unstaged,
        }
    }

    #[test]
    fn git_tools_take_a_closed_set_of_arguments() {
        assert_eq!(request("git_status", "{}"), Ok(Request::GitStatus));
        assert_eq!(
            request("git_status", r#"{"all":true}"#),
            Err("invalid_arguments")
        );
        assert_eq!(
            request("git_diff", r#"{"path":"src/a.rs"}"#),
            Ok(Request::GitDiff {
                source: DiffSource::Unstaged,
                path: "src/a.rs".into()
            })
        );
        assert_eq!(
            request("git_diff", r#"{"path":"a","source":"branch"}"#),
            Ok(Request::GitDiff {
                source: DiffSource::Branch,
                path: "a".into()
            })
        );
        for (arguments, reason) in [
            ("{}", "invalid_arguments"),
            (r#"{"path":""}"#, "path_invalid"),
            (r#"{"path":"a","source":"HEAD~5"}"#, "invalid_arguments"),
            (
                r#"{"path":"a","source":"--output=/x"}"#,
                "invalid_arguments",
            ),
            (r#"{"path":"a","source":1}"#, "invalid_arguments"),
            (r#"{"path":"a","rev":"x"}"#, "invalid_arguments"),
            (r#"{"path":"../a"}"#, "path_outside"),
            (r#"{"path":"--output=x/../../y"}"#, "path_outside"),
            (r#"{"path":".env"}"#, "path_protected"),
            (r#"{"path":".git/config"}"#, "path_reserved"),
        ] {
            assert_eq!(request("git_diff", arguments), Err(reason), "{arguments}");
        }
    }

    #[test]
    fn a_status_lists_changes_hides_ignored_and_protected_entries_and_counts_the_latter() {
        let entries = [
            status_entry(EntryKind::Changed, "a.rs", None, Some(ChangeCode::Modified)),
            status_entry(EntryKind::Changed, "b.rs", Some(ChangeCode::Added), None),
            status_entry(EntryKind::Untracked, "new\nline.txt", None, None),
            status_entry(EntryKind::Untracked, ".env", None, None),
            status_entry(
                EntryKind::Changed,
                "keys/id_rsa",
                Some(ChangeCode::Added),
                None,
            ),
            status_entry(EntryKind::Ignored, "target/", None, None),
            status_entry(EntryKind::Conflicted, "c.rs", None, None),
        ];

        let (text, truncated) = render_status(&entries, true);

        assert_eq!(
            text,
            ".M a.rs\nA. b.rs\n?? new?line.txt\nUU c.rs\n[2 protected entries hidden]\n"
        );
        assert!(!truncated);
        assert!(!text.contains(".env") && !text.contains("id_rsa") && !text.contains("target"));
    }

    #[test]
    fn a_status_without_changes_is_clean_and_an_incomplete_one_says_so() {
        assert_eq!(
            render_status(&[], true),
            ("working tree clean\n".into(), false)
        );
        assert_eq!(
            render_status(&[], false),
            ("working tree clean\n[truncated]\n".into(), true)
        );
    }

    #[test]
    fn a_long_status_is_cut_at_8_kib_on_a_whole_line() {
        let entries: Vec<_> = (0..2_000)
            .map(|index| {
                status_entry(
                    EntryKind::Untracked,
                    &format!("file-{index:05}.txt"),
                    None,
                    None,
                )
            })
            .collect();

        let (text, truncated) = render_status(&entries, true);

        assert!(truncated && text.len() <= MAX_LIST_BYTES);
        assert!(text.ends_with("[truncated]\n"));
    }

    #[test]
    fn a_diff_is_cut_at_32_kib_on_a_whole_line_with_an_explicit_note() {
        let patch = "+x\n".repeat(20_000);

        let (text, truncated) = render_diff(patch.as_bytes(), false);

        assert!(truncated && text.len() <= MAX_DIFF_BYTES);
        assert!(text.ends_with("+x\n[truncated]\n"));
        assert_eq!(render_diff(b"", false), ("no differences\n".into(), false));
        assert_eq!(
            render_diff(b"+a\n", true),
            ("+a\n[truncated]\n".into(), true)
        );
        assert_eq!(render_diff(b"+a\0b", false), ("+ab\n".into(), false));
        assert!(render_diff(&[0xff, b'\n'], false).0.contains('\u{fffd}'));
    }
}
