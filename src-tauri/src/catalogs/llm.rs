use serde::{Deserialize, Serialize, Serializer};

const FILE: &str = include_str!("llm.yml");

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Location {
    Cloud,
    Local,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    AnthropicMessages,
    OpenaiResponses,
    ChatCompletions,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputTokensParam {
    MaxTokens,
    MaxCompletionTokens,
}

impl OutputTokensParam {
    pub fn as_str(self) -> &'static str {
        match self {
            OutputTokensParam::MaxTokens => "max_tokens",
            OutputTokensParam::MaxCompletionTokens => "max_completion_tokens",
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Model {
    pub key: String,
    pub max_output_tokens: u32,
    #[serde(default)]
    pub tools: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Provider {
    pub key: String,
    pub name: String,
    pub location: Location,
    #[serde(skip_serializing)]
    pub protocol: Protocol,
    pub base_url: String,
    #[serde(default, skip_serializing)]
    pub output_tokens_param: Option<OutputTokensParam>,
    #[serde(default, serialize_with = "model_keys")]
    pub models: Vec<Model>,
}

fn model_keys<S: Serializer>(models: &[Model], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(models.iter().map(|model| &model.key))
}

pub fn providers() -> Vec<Provider> {
    serde_yaml_ng::from_str(FILE).expect("embedded llm catalog is valid YAML")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn providers_have_unique_keys_and_models() {
        let providers = providers();
        let models: Vec<&String> = providers
            .iter()
            .flat_map(|entry| entry.models.iter().map(|model| &model.key))
            .collect();

        assert_eq!(
            providers
                .iter()
                .map(|entry| &entry.key)
                .collect::<HashSet<_>>()
                .len(),
            providers.len()
        );
        assert!(
            providers
                .iter()
                .filter(|entry| entry.location == Location::Cloud)
                .all(|entry| !entry.models.is_empty())
        );
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
    fn local_providers_run_on_localhost_and_declare_no_models() {
        assert!(
            providers()
                .iter()
                .filter(|entry| entry.location == Location::Local)
                .all(|entry| entry.base_url.starts_with("http://localhost:")
                    && entry.models.is_empty())
        );
    }

    #[test]
    fn cloud_providers_use_https() {
        assert!(
            providers()
                .iter()
                .filter(|entry| entry.location == Location::Cloud)
                .all(|entry| entry.base_url.starts_with("https://"))
        );
    }

    #[test]
    fn only_chat_completions_declares_the_output_tokens_parameter() {
        for entry in providers() {
            assert_eq!(
                entry.output_tokens_param.is_some(),
                entry.protocol == Protocol::ChatCompletions,
                "{}",
                entry.key
            );
        }
    }

    #[test]
    fn declares_each_protocol_and_documented_output_limit() {
        let providers = providers();
        let find = |key: &str| providers.iter().find(|entry| entry.key == key).unwrap();

        assert_eq!(find("anthropic").protocol, Protocol::AnthropicMessages);
        assert_eq!(find("openai").protocol, Protocol::OpenaiResponses);
        assert_eq!(
            find("groq").output_tokens_param,
            Some(OutputTokensParam::MaxCompletionTokens)
        );
        assert_eq!(
            find("ollama").output_tokens_param,
            Some(OutputTokensParam::MaxTokens)
        );
        assert_eq!(find("groq").models[0].max_output_tokens, 65536);
        assert_eq!(find("deepseek").protocol, Protocol::ChatCompletions);
        assert_eq!(
            find("deepseek").output_tokens_param,
            Some(OutputTokensParam::MaxTokens)
        );
        assert!(
            find("deepseek")
                .models
                .iter()
                .all(|model| model.max_output_tokens == 393216)
        );
        assert!(
            find("openai")
                .models
                .iter()
                .all(|model| model.max_output_tokens == 128000)
        );
    }

    #[test]
    fn the_ui_receives_model_names_without_protocol_details() {
        let json = serde_json::to_value(providers()).unwrap();

        assert_eq!(
            json[1]["models"],
            serde_json::json!(["gpt-6.1-sol", "gpt-6.1-terra"])
        );
        assert!(json[1].get("protocol").is_none());
        assert!(json[2].get("output_tokens_param").is_none());
    }

    #[test]
    fn offers_the_replacements_of_retired_models_for_new_agents() {
        let providers = providers();
        let exists = |provider: &str, model: &str| {
            providers
                .iter()
                .any(|entry| entry.key == provider && entry.models.iter().any(|m| m.key == model))
        };

        assert!(exists("openai", "gpt-6.1-sol"));
        assert!(exists("openai", "gpt-6.1-terra"));
        assert!(exists("groq", "openai/gpt-oss-120b"));
        assert!(exists("deepseek", "deepseek-flash"));
        assert!(exists("deepseek", "deepseek-v4-pro"));
        assert!(!exists("openai", "gpt-5"));
        assert!(!exists("openai", "gpt-5-mini"));
        assert!(!exists("groq", "llama-3.3-70b-versatile"));
    }
}
