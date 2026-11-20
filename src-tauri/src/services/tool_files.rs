use crate::domain::path_jail::is_hidden_entry;
use crate::domain::tools::{ListedKind, Request};
use crate::domain::tools::{
    MAX_EDIT_FILE_BYTES, MAX_LIST_ENTRIES, MAX_SEARCH_DEPTH, MAX_SEARCH_DIRS,
    MAX_SEARCH_FILE_BYTES, MAX_SEARCH_FILES, MAX_SEARCH_TOTAL_BYTES, Needle, READ_WINDOW,
    SearchReport, apply_edit, decode_window, render_listing, render_window,
};
use crate::system::safe_file::{self, SafeFileError};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

pub type ToolResult = Result<(String, bool), &'static str>;

pub fn run(root: &Path, request: Request, stop: &AtomicBool) -> ToolResult {
    match request {
        Request::ListDir { path } => list_dir(root, &path),
        Request::ReadFile { path, offset } => read_file(root, &path, offset),
        Request::Search {
            query,
            path,
            ignore_case,
        } => search(root, &query, &path, ignore_case, stop),
        Request::WriteFile { path, content } => write_file(root, &path, &content),
        Request::EditFile {
            path,
            old_text,
            new_text,
        } => edit_file(root, &path, &old_text, &new_text),
        Request::GitStatus | Request::GitDiff { .. } => Err("internal_error"),
    }
}

fn write_file(root: &Path, path: &str, content: &str) -> ToolResult {
    safe_file::replace(root, path, content.as_bytes()).map_err(file_code_of)?;
    Ok((format!("wrote {path} ({} bytes)\n", content.len()), false))
}

fn edit_file(root: &Path, path: &str, old_text: &str, new_text: &str) -> ToolResult {
    let (chunk, more) = read_chunk(root, path, 0, MAX_EDIT_FILE_BYTES)?;
    if more {
        return Err("file_too_large");
    }
    let content = match String::from_utf8(chunk) {
        Ok(content) if !content.contains('\0') => content,
        _ => return Err("binary_file"),
    };
    let edited = apply_edit(&content, old_text, new_text)?;
    safe_file::replace(root, path, edited.as_bytes()).map_err(file_code_of)?;
    Ok((format!("edited {path}\n"), false))
}

fn code_of(error: SafeFileError) -> &'static str {
    match error {
        SafeFileError::SymbolicLink => "symlink_refused",
        SafeFileError::NotAFile => "not_a_directory",
        SafeFileError::Exists => "io_error",
        SafeFileError::Io(error) => match error.kind() {
            std::io::ErrorKind::NotFound => "not_found",
            std::io::ErrorKind::PermissionDenied => "permission_denied",
            _ => "io_error",
        },
    }
}

fn file_code_of(error: SafeFileError) -> &'static str {
    match code_of(error) {
        "not_a_directory" => "not_a_file",
        other => other,
    }
}

fn list_dir(root: &Path, path: &str) -> ToolResult {
    let (entries, cut) = safe_file::list_dir(root, path, MAX_LIST_ENTRIES).map_err(code_of)?;
    Ok(render_listing(&entries, cut))
}

fn read_chunk(
    root: &Path,
    path: &str,
    offset: u64,
    window: usize,
) -> Result<(Vec<u8>, bool), &'static str> {
    let handle = safe_file::open_for_read(root, path).map_err(file_code_of)?;
    let mut file = &handle.file;
    let length = file.metadata().map_err(|_| "io_error")?.len();
    if offset > length {
        return Err("offset_out_of_range");
    }
    file.seek(SeekFrom::Start(offset)).map_err(|_| "io_error")?;
    let mut chunk = Vec::with_capacity(window + 1);
    file.take(window as u64 + 1)
        .read_to_end(&mut chunk)
        .map_err(|_| "io_error")?;
    let more = chunk.len() > window;
    chunk.truncate(window);
    Ok((chunk, more))
}

fn read_file(root: &Path, path: &str, offset: u64) -> ToolResult {
    let (chunk, more) = read_chunk(root, path, offset, READ_WINDOW)?;
    Ok(render_window(decode_window(&chunk, offset, more)?))
}

#[derive(Default)]
struct Inspected {
    files: u32,
    dirs: u32,
    bytes: usize,
}

fn search(
    root: &Path,
    query: &str,
    start: &str,
    ignore_case: bool,
    stop: &AtomicBool,
) -> ToolResult {
    let needle = Needle::new(query, ignore_case);
    let mut report = SearchReport::default();
    let mut inspected = Inspected::default();
    let mut incomplete = false;
    let mut pending = vec![(start.to_string(), 0usize)];
    let mut first = true;
    while let Some((dir, depth)) = pending.pop() {
        if stop.load(Ordering::Relaxed) {
            return Err("cancelled");
        }
        if inspected.dirs >= MAX_SEARCH_DIRS {
            incomplete = true;
            break;
        }
        inspected.dirs += 1;
        let (entries, cut) = match safe_file::list_dir(root, &dir, MAX_LIST_ENTRIES) {
            Ok(listing) => listing,
            Err(error) if first => return Err(code_of(error)),
            Err(_) => {
                incomplete = true;
                continue;
            }
        };
        first = false;
        incomplete |= cut;
        let mut entries: Vec<_> = entries
            .into_iter()
            .filter(|entry| !is_hidden_entry(&entry.name))
            .collect();
        entries.sort_by(|left, right| left.name.cmp(&right.name));
        let mut subdirs = Vec::new();
        for entry in entries {
            let relative = if dir.is_empty() {
                entry.name.clone()
            } else {
                format!("{dir}/{}", entry.name)
            };
            match entry.kind {
                ListedKind::Directory if depth < MAX_SEARCH_DEPTH => {
                    subdirs.push((relative, depth + 1));
                }
                ListedKind::Directory => incomplete = true,
                ListedKind::File => {
                    if stop.load(Ordering::Relaxed) {
                        return Err("cancelled");
                    }
                    if inspected.files >= MAX_SEARCH_FILES
                        || inspected.bytes >= MAX_SEARCH_TOTAL_BYTES
                    {
                        incomplete = true;
                        pending.clear();
                        subdirs.clear();
                        break;
                    }
                    inspected.files += 1;
                    let Some((content, partial)) = read_text(root, &relative, &mut inspected)
                    else {
                        continue;
                    };
                    incomplete |= partial;
                    report.add_file(&relative, &content, &needle);
                    if report.is_full() {
                        pending.clear();
                        subdirs.clear();
                        break;
                    }
                }
                ListedKind::Symlink | ListedKind::Other => {}
            }
        }
        pending.extend(subdirs.into_iter().rev());
    }
    Ok(report.finish(incomplete))
}

fn read_text(root: &Path, relative: &str, inspected: &mut Inspected) -> Option<(String, bool)> {
    let (chunk, more) = read_chunk(root, relative, 0, MAX_SEARCH_FILE_BYTES).ok()?;
    inspected.bytes += chunk.len();
    let window = decode_window(&chunk, 0, more).ok()?;
    Some((window.text, more))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::tools::{MAX_READ_BYTES, MAX_SEARCH_BYTES};
    use std::fs;

    #[test]
    fn edits_only_text_files_within_the_size_limit_and_leaves_refused_ones_untouched() {
        let root = tree();
        let path = root.path();
        let large = "x".repeat(MAX_EDIT_FILE_BYTES + 1);
        fs::write(path.join("large.txt"), &large).unwrap();
        let edit = |file: &str, old: &str| {
            run(
                path,
                Request::EditFile {
                    path: file.into(),
                    old_text: old.into(),
                    new_text: "X".into(),
                },
                &AtomicBool::new(false),
            )
        };

        assert_eq!(edit("README.md", "world").unwrap().0, "edited README.md\n");
        assert_eq!(edit("blob.bin", "needle"), Err("binary_file"));
        assert_eq!(edit("latin.txt", "needle"), Err("binary_file"));
        assert_eq!(edit("large.txt", "x"), Err("file_too_large"));
        assert_eq!(edit("missing.txt", "x"), Err("not_found"));
        assert_eq!(edit("src", "x"), Err("not_a_file"));
        assert_eq!(
            fs::read_to_string(path.join("README.md")).unwrap(),
            "hello X\nsecond line\n"
        );
        assert_eq!(fs::read_to_string(path.join("large.txt")).unwrap(), large);
    }

    fn tree() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        let path = root.path();
        fs::create_dir_all(path.join("src/nested")).unwrap();
        fs::create_dir_all(path.join(".git")).unwrap();
        fs::write(path.join("README.md"), "hello world\nsecond line\n").unwrap();
        fs::write(path.join("src/lib.rs"), "fn main() {}\n// needle here\n").unwrap();
        fs::write(path.join("src/nested/deep.txt"), "Needle deep\n").unwrap();
        fs::write(path.join(".env"), "API_KEY=needle-secret\n").unwrap();
        fs::write(path.join("src/secrets.json"), "{\"needle\": 1}\n").unwrap();
        fs::write(path.join(".git/config"), "needle in git\n").unwrap();
        fs::write(
            path.join("blob.bin"),
            [0u8, 1, 2, b'n', b'e', b'e', b'd', b'l', b'e'],
        )
        .unwrap();
        fs::write(
            path.join("latin.txt"),
            [0xE9, 0xE8, b'n', b'e', b'e', b'd', b'l', b'e'],
        )
        .unwrap();
        root
    }

    fn read(root: &Path, path: &str, offset: u64) -> ToolResult {
        read_file(root, path, offset)
    }

    fn find(root: &Path, query: &str, path: &str, ignore_case: bool) -> ToolResult {
        search(root, query, path, ignore_case, &AtomicBool::new(false))
    }

    #[test]
    fn reads_a_small_file_whole() {
        let root = tree();

        let (text, truncated) = read(root.path(), "README.md", 0).unwrap();

        assert_eq!(text, "hello world\nsecond line\n");
        assert!(!truncated);
        assert!(read(root.path(), "src/lib.rs", 0).is_ok());
    }

    #[test]
    fn pages_a_long_file_with_offsets_that_never_split_a_character() {
        let root = tempfile::tempdir().unwrap();
        let content: String = "é".repeat(20_000);
        fs::write(root.path().join("long.txt"), &content).unwrap();

        let mut rebuilt = String::new();
        let mut offset = 0u64;
        let mut pages = 0;
        loop {
            let (text, truncated) = read(root.path(), "long.txt", offset).unwrap();
            assert!(text.len() <= MAX_READ_BYTES, "{}", text.len());
            let (body, next) = match text.rsplit_once("[truncated: continue with offset ") {
                Some((body, rest)) => (
                    body.trim_end_matches('\n').to_string(),
                    Some(rest.trim_end_matches("]\n").parse::<u64>().unwrap()),
                ),
                None => (text.clone(), None),
            };
            rebuilt.push_str(&body);
            pages += 1;
            assert_eq!(truncated, next.is_some());
            match next {
                Some(next) => offset = next,
                None => break,
            }
        }

        assert_eq!(rebuilt, content);
        assert!(pages >= 3);
    }

    #[test]
    fn the_first_page_of_a_long_file_stays_within_16_kib_including_the_note() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("big.txt"), "x".repeat(100_000)).unwrap();

        let (text, truncated) = read(root.path(), "big.txt", 0).unwrap();

        assert!(truncated);
        assert!(text.len() <= MAX_READ_BYTES);
        assert!(text.ends_with("]\n"));
    }

    #[test]
    fn refuses_bad_offsets_binaries_and_non_files() {
        let root = tree();
        fs::write(root.path().join("accents.txt"), "ééé").unwrap();

        assert_eq!(read(root.path(), "accents.txt", 1), Err("invalid_offset"));
        assert_eq!(
            read(root.path(), "accents.txt", 99),
            Err("offset_out_of_range")
        );
        assert_eq!(read(root.path(), "blob.bin", 0), Err("binary_file"));
        assert_eq!(read(root.path(), "latin.txt", 0), Err("binary_file"));
        assert_eq!(read(root.path(), "src", 0), Err("not_a_file"));
        assert_eq!(read(root.path(), "missing.txt", 0), Err("not_found"));
        assert_eq!(read(root.path(), "README.md", 24).unwrap().0, "");
    }

    #[cfg(unix)]
    #[test]
    fn never_follows_a_symbolic_link_in_the_file_or_any_folder() {
        let root = tree();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret.txt"), "outside needle\n").unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("secret.txt"),
            root.path().join("link.txt"),
        )
        .unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("linked")).unwrap();

        assert!(read(root.path(), "link.txt", 0).is_err());
        assert!(read(root.path(), "linked/secret.txt", 0).is_err());
        let (text, _) = find(root.path(), "outside", "", false).unwrap();
        assert_eq!(text, "no matches\n");
    }

    #[cfg(unix)]
    #[test]
    fn a_folder_swapped_for_a_link_after_listing_is_refused_when_read() {
        let root = tree();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("lib.rs"), "stolen\n").unwrap();
        let (entries, _) = safe_file::list_dir(root.path(), "", 100).unwrap();
        assert!(entries.iter().any(|entry| entry.name == "src"));

        fs::remove_dir_all(root.path().join("src")).unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("src")).unwrap();

        assert!(read(root.path(), "src/lib.rs", 0).is_err());
        assert!(find(root.path(), "stolen", "src", false).is_err());
    }

    #[test]
    fn search_finds_text_in_order_and_skips_git_secrets_and_binaries() {
        let root = tree();

        let (text, truncated) = find(root.path(), "needle", "", false).unwrap();

        assert_eq!(text, "src/lib.rs:2: // needle here\n");
        assert!(!truncated);
        let (text, _) = find(root.path(), "needle", "", true).unwrap();
        assert_eq!(
            text,
            "src/lib.rs:2: // needle here\nsrc/nested/deep.txt:1: Needle deep\n"
        );
        assert!(!text.contains("secret") && !text.contains(".git"));
    }

    #[test]
    fn search_can_start_in_a_folder_and_reports_missing_ones() {
        let root = tree();

        assert_eq!(
            find(root.path(), "Needle", "src/nested", false).unwrap().0,
            "src/nested/deep.txt:1: Needle deep\n"
        );
        assert_eq!(find(root.path(), "x", "nope", false), Err("not_found"));
        assert_eq!(
            find(root.path(), "x", "README.md", false),
            Err("not_a_directory")
        );
    }

    #[test]
    fn search_output_is_capped_at_8_kib_with_a_truncation_note() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("many.txt"), "needle\n".repeat(5_000)).unwrap();

        let (text, truncated) = find(root.path(), "needle", "", false).unwrap();

        assert!(truncated);
        assert!(text.len() <= MAX_SEARCH_BYTES);
        assert!(text.ends_with("[truncated]\n"));
    }

    #[test]
    fn search_stops_after_the_file_limit_and_says_it_is_incomplete() {
        let root = tempfile::tempdir().unwrap();
        for folder in 0..5 {
            let folder = root.path().join(format!("d{folder}"));
            fs::create_dir(&folder).unwrap();
            for index in 0..(MAX_SEARCH_FILES / 4) {
                fs::write(folder.join(format!("f{index:05}.txt")), "x\n").unwrap();
            }
        }
        fs::create_dir(root.path().join("z")).unwrap();
        fs::write(root.path().join("z/last.txt"), "needle\n").unwrap();

        let (text, truncated) = find(root.path(), "needle", "", false).unwrap();

        assert!(truncated);
        assert_eq!(text, "[truncated]\n");
    }

    #[test]
    fn search_stops_after_the_byte_limit() {
        let root = tempfile::tempdir().unwrap();
        let line = "y".repeat(100) + "\n";
        let content = line.repeat(MAX_SEARCH_FILE_BYTES / 101);
        for index in 0..40 {
            fs::write(root.path().join(format!("b{index:02}.txt")), &content).unwrap();
        }

        let (text, truncated) = find(root.path(), "needle", "", false).unwrap();

        assert!(truncated, "{text}");
    }

    #[test]
    fn search_only_reads_the_start_of_a_huge_file_and_says_so() {
        let root = tempfile::tempdir().unwrap();
        let mut content = "a\n".repeat(MAX_SEARCH_FILE_BYTES);
        content.push_str("needle\n");
        fs::write(root.path().join("huge.txt"), content).unwrap();

        let (text, truncated) = find(root.path(), "needle", "", false).unwrap();

        assert!(truncated);
        assert!(!text.contains("needle"));
    }

    #[test]
    fn search_depth_is_bounded() {
        let root = tempfile::tempdir().unwrap();
        let mut path = root.path().to_path_buf();
        for _ in 0..(MAX_SEARCH_DEPTH + 3) {
            path.push("d");
        }
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("deep.txt"), "needle\n").unwrap();

        let (text, truncated) = find(root.path(), "needle", "", false).unwrap();

        assert!(truncated);
        assert_eq!(text, "[truncated]\n");
    }

    #[test]
    fn a_raised_stop_flag_ends_the_search() {
        let root = tree();
        let stop = AtomicBool::new(true);

        assert_eq!(
            search(root.path(), "needle", "", false, &stop),
            Err("cancelled")
        );
    }

    #[test]
    fn listing_still_hides_git_and_secrets() {
        let root = tree();

        let (text, _) = list_dir(root.path(), "").unwrap();

        assert!(text.contains("src/") && text.contains("README.md"));
        assert!(!text.contains(".env") && !text.contains(".git"));
    }
}
