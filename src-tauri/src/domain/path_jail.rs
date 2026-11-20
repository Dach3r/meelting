use crate::domain::review::protected_path;

const MAX_PATH_BYTES: usize = 1024;
const MAX_COMPONENT_BYTES: usize = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathError {
    Invalid,
    Outside,
    Reserved,
    Protected,
}

impl PathError {
    pub fn code(self) -> &'static str {
        match self {
            PathError::Invalid => "path_invalid",
            PathError::Outside => "path_outside",
            PathError::Reserved => "path_reserved",
            PathError::Protected => "path_protected",
        }
    }
}

pub fn parse(path: &str) -> Result<String, PathError> {
    if path.len() > MAX_PATH_BYTES
        || path
            .chars()
            .any(|character| character.is_control() || matches!(character, '\\' | ':'))
    {
        return Err(PathError::Invalid);
    }
    if path.starts_with('/') {
        return Err(PathError::Outside);
    }
    let mut components = Vec::new();
    for component in path.split('/') {
        match component {
            "" | "." => continue,
            ".." => return Err(PathError::Outside),
            _ => {}
        }
        if component.len() > MAX_COMPONENT_BYTES || component.ends_with(['.', ' ']) {
            return Err(PathError::Invalid);
        }
        if reserved(component) {
            return Err(PathError::Reserved);
        }
        components.push(component);
    }
    let normalized = components.join("/");
    if protected_path(normalized.as_bytes()) {
        return Err(PathError::Protected);
    }
    Ok(normalized)
}

fn reserved(component: &str) -> bool {
    let lower = component.to_lowercase();
    if lower == ".git" {
        return true;
    }
    let stem = lower.split('.').next().unwrap_or_default();
    if let Some((head, digits)) = stem.rsplit_once('~')
        && !head.is_empty()
        && !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return true;
    }
    matches!(stem, "con" | "prn" | "aux" | "nul" | "conin$" | "conout$")
        || stem
            .strip_prefix("com")
            .or_else(|| stem.strip_prefix("lpt"))
            .is_some_and(|number| {
                matches!(number, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            })
}

pub fn is_hidden_entry(name: &str) -> bool {
    reserved(name) || protected_path(name.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_relative_paths_inside_the_root() {
        assert_eq!(parse("").unwrap(), "");
        assert_eq!(parse(".").unwrap(), "");
        assert_eq!(parse("./src//lib/").unwrap(), "src/lib");
        assert_eq!(parse("src/main.rs").unwrap(), "src/main.rs");
    }

    #[test]
    fn refuses_every_way_out_of_the_root() {
        for path in ["/etc", "..", "a/../b", "a/..", "../a", "/"] {
            assert_eq!(parse(path), Err(PathError::Outside), "{path}");
        }
        for path in [
            "C:\\x",
            "C:/x",
            "a\\b",
            "\\\\server\\share",
            "a:stream",
            "a\0b",
            "a\nb",
        ] {
            assert_eq!(parse(path), Err(PathError::Invalid), "{path:?}");
        }
    }

    #[test]
    fn refuses_git_internals_and_device_names_in_any_case_or_extension() {
        for path in [
            ".git",
            ".GIT/config",
            "a/.Git",
            "git~1",
            "GIT~12/hooks",
            "CON",
            "nul.txt",
            "a/Com1",
            "lpt9.log",
            "AUX.tar.gz",
            "CONIN$",
        ] {
            assert_eq!(parse(path), Err(PathError::Reserved), "{path}");
        }
        for short in ["CREDEN~1.JSO", "PROGRA~1", "ENV~1", "a~22.txt", "x~9"] {
            assert_eq!(parse(short), Err(PathError::Reserved), "{short}");
        }
        assert!(parse("git~").is_ok());
        assert!(parse("~1").is_ok());
        assert!(parse("tilde~name").is_ok());
        assert!(parse("console").is_ok());
        assert!(parse("com10").is_ok());
        assert!(parse(".gitignore").is_ok());
    }

    #[test]
    fn refuses_trailing_dots_and_spaces_that_windows_would_strip() {
        for path in [".git.", "a/b ", "src./x", "x.."] {
            assert!(parse(path).is_err(), "{path}");
        }
    }

    #[test]
    fn refuses_secret_names_anywhere_in_the_path() {
        for path in [
            ".env",
            "config/.env.local",
            "keys/id_rsa",
            "a/credentials.json",
        ] {
            assert_eq!(parse(path), Err(PathError::Protected), "{path}");
        }
    }

    #[test]
    fn bounds_the_length_of_paths_and_components() {
        assert_eq!(parse(&"a/".repeat(600)), Err(PathError::Invalid));
        assert_eq!(parse(&"a".repeat(256)), Err(PathError::Invalid));
        assert!(parse(&"a".repeat(255)).is_ok());
    }

    #[test]
    fn hides_reserved_and_secret_entries_from_listings() {
        assert!(is_hidden_entry(".git"));
        assert!(is_hidden_entry(".env"));
        assert!(!is_hidden_entry("src"));
    }
}
