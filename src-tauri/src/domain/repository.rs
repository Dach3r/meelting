use serde::Serialize;
use std::path::Path;

pub const NAME_MAX_CHARS: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Repository {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepositoryBranches {
    pub branches: Vec<String>,
    pub default_branch: Option<String>,
}

pub fn name_from_path(path: &str) -> String {
    let name = Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_string());
    name.chars().take(NAME_MAX_CHARS).collect()
}

pub fn default_branch(
    origin_head: Option<&str>,
    head: Option<&str>,
    branches: &[String],
) -> Option<String> {
    let local = |name: &str| branches.iter().any(|branch| branch == name);
    origin_head
        .and_then(|reference| reference.strip_prefix("refs/remotes/origin/"))
        .filter(|name| local(name))
        .or_else(|| {
            head.and_then(|reference| reference.strip_prefix("refs/heads/"))
                .filter(|name| local(name))
        })
        .map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn branches(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn names_the_repository_after_its_folder() {
        assert_eq!(name_from_path("/Users/ana/Developer/my-app"), "my-app");
        assert_eq!(name_from_path("C:/Users/ana/mi app ñ"), "mi app ñ");
    }

    #[test]
    fn falls_back_to_the_whole_path_without_a_folder_name() {
        assert_eq!(name_from_path("/"), "/");
    }

    #[test]
    fn caps_the_name_length() {
        let path = format!("/repos/{}", "ñ".repeat(NAME_MAX_CHARS + 5));

        assert_eq!(name_from_path(&path).chars().count(), NAME_MAX_CHARS);
    }

    #[test]
    fn prefers_the_remote_default_branch_when_it_exists_locally() {
        let suggested = default_branch(
            Some("refs/remotes/origin/main"),
            Some("refs/heads/feature"),
            &branches(&["feature", "main"]),
        );

        assert_eq!(suggested.as_deref(), Some("main"));
    }

    #[test]
    fn falls_back_to_the_current_branch() {
        let all = branches(&["develop", "feature"]);

        assert_eq!(
            default_branch(
                Some("refs/remotes/origin/main"),
                Some("refs/heads/feature"),
                &all
            )
            .as_deref(),
            Some("feature")
        );
        assert_eq!(
            default_branch(None, Some("refs/heads/develop"), &all).as_deref(),
            Some("develop")
        );
    }

    #[test]
    fn suggests_nothing_with_a_detached_head_and_no_remote() {
        assert_eq!(default_branch(None, None, &branches(&["main"])), None);
    }
}
