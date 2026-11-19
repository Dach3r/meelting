use crate::catalogs::llm::{Location, Provider};
use crate::domain::agent_run::ErrorKind;
use crate::domain::prompt::{Prompt, Role, Turn};
use crate::domain::provider_key::{self, Verification};
use crate::services::agent_run_engine::Engine;
use crate::services::session_load::{self, SessionError};
use crate::system::keychain;
use crate::system::llm_call;
use crate::system::llm_stream::StreamRequest;
use serde::Serialize;

const VERIFY_OUTPUT_TOKENS: u32 = 16;
const VERIFY_INSTRUCTIONS: &str = "Reply with OK.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderKeyResult {
    pub verification: Verification,
    pub last_four: Option<String>,
}

pub async fn target<'a>(
    engine: &'a Engine,
    provider: &str,
) -> Result<(&'a Provider, String), SessionError> {
    let found = engine
        .providers
        .iter()
        .find(|entry| entry.key == provider && entry.location == Location::Cloud)
        .ok_or(SessionError::InvalidProvider)?;
    let workspace_id = session_load::require(&engine.pool)
        .await?
        .current_workspace_id;
    Ok((found, provider_key::secret_name(&workspace_id, &found.key)))
}

pub async fn verify(engine: &Engine, provider: &Provider, key: &str) -> Verification {
    let Some(model) = provider.models.first() else {
        return Verification::Rejected {
            reason: ErrorKind::ModelUnavailable,
        };
    };
    let prompt = Prompt {
        system: VERIFY_INSTRUCTIONS.into(),
        turns: vec![Turn {
            role: Role::User,
            text: "OK".into(),
        }],
    };
    let request = StreamRequest {
        base_url: &provider.base_url,
        api_key: Some(key),
        model: &model.key,
        max_output_tokens: VERIFY_OUTPUT_TOKENS,
        prompt: &prompt,
        connect_timeout: engine.limits.cloud.connect_timeout,
        idle_timeout: engine.limits.cloud.idle_timeout,
    };
    let verification = match llm_call::stream(provider, &request, &mut |_| Ok(())).await {
        Ok(_) => Verification::Verified,
        Err(failure) => provider_key::from_failure(failure.kind, failure.unreachable),
    };
    engine
        .registry
        .reached(&provider.key, verification != Verification::Offline);
    verification
}

pub async fn run(engine: &Engine, provider: &str) -> Result<ProviderKeyResult, SessionError> {
    let (found, name) = target(engine, provider).await?;
    let key = keychain::read(&name)?.ok_or(SessionError::ProviderNotConfigured)?;
    let verification = verify(engine, found, &key).await;
    Ok(ProviderKeyResult {
        verification,
        last_four: keychain::last_four(&name)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::agent_run_engine::testing::{chat, fast_limits, harness};
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn server(response: ResponseTemplate, calls: u64) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(response)
            .expect(calls)
            .mount(&server)
            .await;
        server
    }

    #[tokio::test]
    async fn checks_the_stored_key_and_clears_the_offline_mark() {
        let server = server(chat(&["OK"]), 1).await;
        let h = harness(&server.uri(), fast_limits(), &[]).await;
        let (_, name) = target(&h.engine, "groq").await.unwrap();
        keychain::save(&name, "sk-stored-9876").unwrap();
        h.engine.registry.reached("groq", false);

        let checked = run(&h.engine, "groq").await;
        keychain::remove(&name).unwrap();

        assert_eq!(
            checked.unwrap(),
            ProviderKeyResult {
                verification: Verification::Verified,
                last_four: Some("9876".into()),
            }
        );
        assert!(!h.engine.registry.is_unreachable("groq"));
    }

    #[tokio::test]
    async fn a_rejected_stored_key_is_reported_but_never_deleted() {
        let server = server(ResponseTemplate::new(401), 1).await;
        let h = harness(&server.uri(), fast_limits(), &[]).await;
        let (_, name) = target(&h.engine, "openai").await.unwrap();
        keychain::save(&name, "sk-revoked-5555").unwrap();

        let checked = run(&h.engine, "openai").await.unwrap();
        let kept = keychain::read(&name).unwrap();
        keychain::remove(&name).unwrap();

        assert_eq!(
            checked.verification,
            Verification::Rejected {
                reason: ErrorKind::ProviderAuthFailed
            }
        );
        assert_eq!(kept.as_deref(), Some("sk-revoked-5555"));
    }

    #[tokio::test]
    async fn checking_without_a_stored_key_makes_no_request() {
        let server = server(chat(&["OK"]), 0).await;
        let h = harness(&server.uri(), fast_limits(), &[]).await;

        assert!(matches!(
            run(&h.engine, "anthropic").await,
            Err(SessionError::ProviderNotConfigured)
        ));
    }

    #[tokio::test]
    async fn only_cloud_providers_of_the_catalog_have_keys() {
        let server = server(chat(&["OK"]), 0).await;
        let h = harness(&server.uri(), fast_limits(), &[]).await;

        for provider in ["ollama", "lmstudio", "minimax", "", "../groq"] {
            assert!(matches!(
                run(&h.engine, provider).await,
                Err(SessionError::InvalidProvider)
            ));
        }
    }
}
