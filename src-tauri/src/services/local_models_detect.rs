use crate::catalogs::llm::{self, Location, Provider};
use crate::domain::local_provider::LocalProviderModels;
use crate::system::local_models;
use futures_util::future::join_all;

pub async fn run() -> Vec<LocalProviderModels> {
    detect(llm::providers()).await
}

async fn detect(providers: Vec<Provider>) -> Vec<LocalProviderModels> {
    join_all(
        providers
            .into_iter()
            .filter(|provider| provider.location == Location::Local)
            .map(|provider| (provider.key, provider.base_url))
            .map(|(key, url)| async move {
                let models = local_models::list(&url).await;
                LocalProviderModels {
                    key,
                    running: models.is_ok(),
                    models: models.unwrap_or_default(),
                }
            }),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogs::llm::{Model, OutputTokensParam, Protocol};
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn local(key: &str, base_url: String) -> Provider {
        Provider {
            key: key.into(),
            name: key.into(),
            location: Location::Local,
            protocol: Protocol::ChatCompletions,
            base_url,
            output_tokens_param: Some(OutputTokensParam::MaxTokens),
            models: vec![],
        }
    }

    #[tokio::test]
    async fn reports_running_and_stopped_local_providers() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "data": [{ "id": "qwen" }] })),
            )
            .mount(&server)
            .await;
        let cloud = Provider {
            key: "openai".into(),
            name: "OpenAI".into(),
            location: Location::Cloud,
            protocol: Protocol::OpenaiResponses,
            base_url: "https://api.openai.com/v1".into(),
            output_tokens_param: None,
            models: vec![Model {
                key: "gpt-6.1-sol".into(),
                max_output_tokens: 128000,
            }],
        };

        let detected = detect(vec![
            cloud,
            local("up", server.uri()),
            local("down", "http://127.0.0.1:1".into()),
        ])
        .await;

        assert_eq!(
            detected,
            [
                LocalProviderModels {
                    key: "up".into(),
                    running: true,
                    models: vec!["qwen".into()],
                },
                LocalProviderModels {
                    key: "down".into(),
                    running: false,
                    models: vec![],
                },
            ]
        );
    }
}
