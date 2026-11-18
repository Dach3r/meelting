use serde::{Deserialize, Serialize};

const FILE: &str = include_str!("agent_roles.yml");

#[derive(Debug, Deserialize, Serialize)]
pub struct RoleGroup {
    pub group: String,
    pub roles: Vec<String>,
}

pub fn groups() -> Vec<RoleGroup> {
    serde_yaml_ng::from_str(FILE).expect("embedded agent roles are valid YAML")
}

pub fn exists(role: &str) -> bool {
    groups()
        .iter()
        .any(|group| group.roles.iter().any(|candidate| candidate == role))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogs::agent_templates;
    use crate::domain::locale::Locale;
    use crate::domain::team::TEXT_MAX_CHARS;
    use std::collections::HashSet;

    #[test]
    fn loads_the_roles_without_duplicates() {
        let roles: Vec<String> = groups().into_iter().flat_map(|group| group.roles).collect();

        assert_eq!(roles.len(), 25);
        assert_eq!(roles.iter().collect::<HashSet<_>>().len(), roles.len());
        assert!(roles.iter().all(|role| {
            role == role.trim() && (1..=TEXT_MAX_CHARS).contains(&role.chars().count())
        }));
    }

    #[test]
    fn groups_have_unique_keys_and_at_least_one_role() {
        let groups = groups();

        assert_eq!(groups.len(), 5);
        assert_eq!(
            groups
                .iter()
                .map(|group| &group.group)
                .collect::<HashSet<_>>()
                .len(),
            groups.len()
        );
        assert!(groups.iter().all(|group| !group.roles.is_empty()));
    }

    #[test]
    fn knows_which_roles_exist() {
        assert!(exists("Tech Lead"));
        assert!(!exists("tech lead"));
        assert!(!exists("Wizard"));
    }

    #[test]
    fn every_template_role_is_a_listed_role() {
        for locale in [Locale::Es, Locale::En] {
            for template in agent_templates::all(locale) {
                assert!(exists(&template.role), "{}", template.role);
            }
        }
    }
}
