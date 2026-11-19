use crate::catalogs::llm::{Location, Provider};
use crate::system::local_models;

pub async fn run(providers: &[Provider], provider: &str, model: &str) -> bool {
    let Some(entry) = providers.iter().find(|entry| entry.key == provider) else {
        return false;
    };
    match entry.location {
        Location::Cloud => entry.models.iter().any(|candidate| candidate.key == model),
        Location::Local => local_models::list(&entry.base_url)
            .await
            .is_ok_and(|installed| installed.iter().any(|candidate| candidate == model)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::agent_run_engine::testing::providers_at;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn cloud_models_must_be_in_the_catalog() {
        let providers = providers_at("http://127.0.0.1:1");

        assert!(run(&providers, "anthropic", "claude-sonnet-5-5").await);
        assert!(!run(&providers, "anthropic", "gpt-6.1-sol").await);
        assert!(!run(&providers, "nobody", "claude-sonnet-5-5").await);
    }

    #[tokio::test]
    async fn local_models_must_be_installed_in_a_running_server() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "data": [{ "id": "qwen2.5:0.5b" }] })),
            )
            .mount(&server)
            .await;
        let running = providers_at(&server.uri());
        let stopped = providers_at("http://127.0.0.1:1");

        assert!(run(&running, "ollama", "qwen2.5:0.5b").await);
        assert!(!run(&running, "ollama", "qwen2.5-coder:32b").await);
        assert!(!run(&running, "ollama", " ").await);
        assert!(!run(&stopped, "ollama", "qwen2.5:0.5b").await);
    }
}
