use serde::Serialize;
use std::path::{Component, Path};

pub const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;
pub const MAX_TOTAL_BYTES: u64 = 50 * 1024 * 1024;
const MAX_PATH_CHARS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyProblem {
    OutsideRepository,
    GitInternal,
    SymbolicLink,
    NotAFile,
    Tracked,
    TooLarge,
    UnsupportedName,
    AlreadyInEnvironment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileToCopy {
    pub relative_path: String,
    pub size_bytes: u64,
    pub secret: bool,
    pub problem: Option<CopyProblem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CopyPreview {
    pub id: String,
    pub files: Vec<FileToCopy>,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyOutcome {
    Copied,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CopyResult {
    pub relative_path: String,
    pub outcome: CopyOutcome,
    pub problem: Option<CopyProblem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileCopy {
    pub relative_path: String,
    pub size_bytes: u64,
    pub copied_at: String,
}

pub fn relative_path(relative: &Path) -> Result<String, CopyProblem> {
    let mut parts = Vec::new();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            return Err(CopyProblem::OutsideRepository);
        };
        let part = part.to_str().ok_or(CopyProblem::UnsupportedName)?;
        if part.eq_ignore_ascii_case(".git") {
            return Err(CopyProblem::GitInternal);
        }
        if part.contains(['/', '\\', ':']) {
            return Err(CopyProblem::UnsupportedName);
        }
        parts.push(part);
    }
    let joined = parts.join("/");
    if joined.is_empty() {
        return Err(CopyProblem::NotAFile);
    }
    if joined.chars().count() > MAX_PATH_CHARS {
        return Err(CopyProblem::UnsupportedName);
    }
    Ok(joined)
}

pub fn looks_secret(relative_path: &str) -> bool {
    let name = relative_path
        .rsplit('/')
        .next()
        .unwrap_or(relative_path)
        .to_lowercase();
    name.starts_with(".env")
        || name.starts_with("id_")
        || name.contains("secret")
        || name.contains("credential")
        || name.contains("token")
        || [".pem", ".key", ".p12", ".pfx", ".keystore", ".jks"]
            .iter()
            .any(|extension| name.ends_with(extension))
        || [".npmrc", ".pypirc", ".netrc", ".htpasswd"].contains(&name.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_portable_relative_paths() {
        assert_eq!(relative_path(Path::new(".env")).unwrap(), ".env");
        assert_eq!(
            relative_path(Path::new("config/local settings ñ.json")).unwrap(),
            "config/local settings ñ.json"
        );
    }

    #[test]
    fn rejects_paths_that_escape_or_touch_git_internals() {
        assert_eq!(
            relative_path(Path::new("../secret")),
            Err(CopyProblem::OutsideRepository)
        );
        assert_eq!(
            relative_path(Path::new("/etc/passwd")),
            Err(CopyProblem::OutsideRepository)
        );
        assert_eq!(
            relative_path(Path::new("a/./b")).unwrap(),
            "a/b",
            "std drops interior current-dir components"
        );
        assert_eq!(
            relative_path(Path::new("vendor/.GIT/config")),
            Err(CopyProblem::GitInternal)
        );
        assert_eq!(
            relative_path(Path::new("notes:stream")),
            Err(CopyProblem::UnsupportedName)
        );
        assert_eq!(relative_path(Path::new("")), Err(CopyProblem::NotAFile));
        assert_eq!(
            relative_path(&Path::new("a").join("b".repeat(1100))),
            Err(CopyProblem::UnsupportedName)
        );
    }

    #[test]
    fn warns_about_files_that_usually_hold_secrets() {
        for name in [
            ".env",
            "config/.env.local",
            "id_ed25519",
            "certs/server.pem",
            "client_secret.json",
            "aws-credentials",
            ".npmrc",
        ] {
            assert!(looks_secret(name), "{name}");
        }
        for name in ["notes.txt", "settings.json", "README.md"] {
            assert!(!looks_secret(name), "{name}");
        }
    }
}
