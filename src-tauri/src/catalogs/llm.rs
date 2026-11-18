use serde::{Deserialize, Serialize};

const FILE: &str = include_str!("llm.yml");

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Location {
    Cloud,
    Local,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Provider {
    pub key: String,
    pub name: String,
    pub location: Location,
    pub models: Vec<String>,
}

pub fn providers() -> Vec<Provider> {
    serde_yaml_ng::from_str(FILE).expect("embedded llm catalog is valid YAML")
}

pub fn exists(provider: &str, model: &str) -> bool {
    providers().iter().any(|entry| {
        entry.key == provider && entry.models.iter().any(|candidate| candidate == model)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn providers_have_unique_keys_and_models() {
        let providers = providers();
        let models: Vec<&String> = providers.iter().flat_map(|entry| &entry.models).collect();

        assert_eq!(
            providers
                .iter()
                .map(|entry| &entry.key)
                .collect::<HashSet<_>>()
                .len(),
            providers.len()
        );
        assert!(providers.iter().all(|entry| !entry.models.is_empty()));
        assert_eq!(models.iter().collect::<HashSet<_>>().len(), models.len());
    }

    #[test]
    fn offers_cloud_and_local_providers() {
        let providers = providers();

        assert!(
            providers
                .iter()
                .any(|entry| entry.location == Location::Cloud)
        );
        assert!(
            providers
                .iter()
                .any(|entry| entry.location == Location::Local)
        );
    }

    #[test]
    fn knows_which_pairs_exist() {
        assert!(exists("anthropic", "claude-sonnet-5-5"));
        assert!(!exists("anthropic", "gpt-5"));
        assert!(!exists("nobody", "claude-sonnet-5-5"));
    }
}
