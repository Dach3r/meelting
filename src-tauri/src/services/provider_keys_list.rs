use crate::catalogs::llm::Location;
use crate::domain::provider_key;
use crate::services::agent_run_engine::Engine;
use crate::services::session_load::{self, SessionError};
use crate::system::keychain;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderKeyStatus {
    pub provider: String,
    pub name: String,
    pub last_four: Option<String>,
    pub offline: bool,
}

pub async fn run(engine: &Engine) -> Result<Vec<ProviderKeyStatus>, SessionError> {
    let workspace_id = session_load::require(&engine.pool)
        .await?
        .current_workspace_id;
    engine
        .providers
        .iter()
        .filter(|provider| provider.location == Location::Cloud)
        .map(|provider| {
            Ok(ProviderKeyStatus {
                provider: provider.key.clone(),
                name: provider.name.clone(),
                last_four: keychain::last_four(&provider_key::secret_name(
                    &workspace_id,
                    &provider.key,
                ))?,
                offline: engine.registry.is_unreachable(&provider.key),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::agent_run_engine::testing::{chat, fast_limits, harness};
    use crate::services::provider_key_check;
    use wiremock::matchers::any;
    use wiremock::{Mock, MockServer};

    #[tokio::test]
    async fn lists_cloud_providers_with_their_last_four_and_offline_mark_without_network() {
        let server = MockServer::start().await;
        Mock::given(any())
            .respond_with(chat(&["OK"]))
            .expect(0)
            .mount(&server)
            .await;
        let h = harness(&server.uri(), fast_limits(), &[]).await;
        let (_, name) = provider_key_check::target(&h.engine, "openai")
            .await
            .unwrap();
        keychain::save(&name, "sk-openai-2468").unwrap();
        h.engine.registry.reached("groq", false);

        let listed = run(&h.engine).await;
        keychain::remove(&name).unwrap();

        let listed = listed.unwrap();
        assert_eq!(
            listed
                .iter()
                .map(|status| status.provider.as_str())
                .collect::<Vec<_>>(),
            ["anthropic", "openai", "groq", "deepseek"]
        );
        assert_eq!(listed[0].last_four, None);
        assert_eq!(listed[1].last_four.as_deref(), Some("2468"));
        assert_eq!(listed[1].name, "OpenAI");
        assert!(!listed[1].offline);
        assert!(listed[2].offline);
    }
}
