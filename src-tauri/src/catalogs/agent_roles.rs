const FILE: &str = include_str!("agent_roles.yml");

pub fn all() -> Vec<String> {
    serde_yaml_ng::from_str(FILE).expect("embedded agent roles are valid YAML")
}

pub fn exists(role: &str) -> bool {
    all().iter().any(|candidate| candidate == role)
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
        let roles = all();

        assert_eq!(roles.len(), 14);
        assert_eq!(roles.iter().collect::<HashSet<_>>().len(), roles.len());
        assert!(roles.iter().all(|role| {
            role == role.trim() && (1..=TEXT_MAX_CHARS).contains(&role.chars().count())
        }));
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
