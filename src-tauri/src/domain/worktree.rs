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
}
