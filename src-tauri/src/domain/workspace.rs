use serde::Serialize;

pub const NAME_MAX_CHARS: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
}

pub fn normalize_name(raw: &str) -> Option<String> {
    let name = raw.trim();
    let length = name.chars().count();
    (1..=NAME_MAX_CHARS)
        .contains(&length)
        .then(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(normalize_name("  Acme  "), Some("Acme".to_string()));
    }

    #[test]
    fn rejects_blank_names() {
        assert_eq!(normalize_name("   "), None);
        assert_eq!(normalize_name(""), None);
    }

    #[test]
    fn accepts_exactly_the_maximum_length() {
        assert!(normalize_name(&"ñ".repeat(NAME_MAX_CHARS)).is_some());
    }

    #[test]
    fn rejects_names_over_the_maximum_length() {
        assert_eq!(normalize_name(&"a".repeat(NAME_MAX_CHARS + 1)), None);
    }
}
