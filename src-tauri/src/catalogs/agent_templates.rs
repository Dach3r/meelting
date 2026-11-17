use crate::domain::locale::Locale;
use serde::{Deserialize, Serialize};

const FILES: [&str; 5] = [
    include_str!("agent_templates/product_owner.yml"),
    include_str!("agent_templates/software_developer.yml"),
    include_str!("agent_templates/qa_engineer.yml"),
    include_str!("agent_templates/devops_engineer.yml"),
    include_str!("agent_templates/project_manager.yml"),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentTemplate {
    pub key: String,
    pub name: String,
    pub role: String,
    pub description: String,
    pub instructions: String,
}

#[derive(Deserialize)]
struct Entry {
    key: String,
    es: Content,
    en: Content,
}

#[derive(Deserialize)]
struct Content {
    name: String,
    role: String,
    description: String,
    instructions: String,
}

fn entries() -> impl Iterator<Item = Entry> {
    FILES
        .iter()
        .map(|file| serde_yaml_ng::from_str(file).expect("embedded agent template is valid YAML"))
}

pub fn all(locale: Locale) -> Vec<AgentTemplate> {
    entries()
        .map(|entry| {
            let content = match locale {
                Locale::Es => entry.es,
                Locale::En => entry.en,
            };
            AgentTemplate {
                key: entry.key,
                name: content.name,
                role: content.role,
                description: content.description,
                instructions: content.instructions,
            }
        })
        .collect()
}

pub fn exists(key: &str) -> bool {
    entries().any(|entry| entry.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn loads_the_five_templates_in_both_languages() {
        for locale in [Locale::Es, Locale::En] {
            let templates = all(locale);
            assert_eq!(templates.len(), 5);
            assert!(templates.iter().all(|template| {
                [
                    &template.name,
                    &template.role,
                    &template.description,
                    &template.instructions,
                ]
                .iter()
                .all(|text| !text.trim().is_empty())
            }));
        }
    }

    #[test]
    fn keys_are_unique() {
        let keys: HashSet<_> = all(Locale::En).into_iter().map(|t| t.key).collect();
        assert_eq!(keys.len(), 5);
    }

    #[test]
    fn content_follows_the_locale() {
        let es = all(Locale::Es);
        let en = all(Locale::En);
        assert_eq!(es[0].key, en[0].key);
        assert_ne!(es[0].description, en[0].description);
    }

    #[test]
    fn respects_the_agent_limits() {
        use crate::domain::team::{DESCRIPTION_MAX_CHARS, INSTRUCTIONS_MAX_CHARS, TEXT_MAX_CHARS};
        for template in all(Locale::Es).into_iter().chain(all(Locale::En)) {
            assert!(template.name.chars().count() <= TEXT_MAX_CHARS);
            assert!(template.role.chars().count() <= TEXT_MAX_CHARS);
            assert!(template.description.chars().count() <= DESCRIPTION_MAX_CHARS);
            assert!(template.instructions.chars().count() <= INSTRUCTIONS_MAX_CHARS);
        }
    }

    #[test]
    fn knows_which_keys_exist() {
        assert!(exists("product_owner"));
        assert!(!exists("missing"));
    }
}
