use crate::domain::provider_key::{self, Verification};
use crate::services::agent_run_engine::Engine;
use crate::services::provider_key_check::{self, ProviderKeyResult};
use crate::services::session_load::SessionError;
use crate::system::keychain;

pub async fn run(
    engine: &Engine,
    provider: &str,
    key: &str,
) -> Result<ProviderKeyResult, SessionError> {
    let (found, name) = provider_key_check::target(engine, provider).await?;
    let key = provider_key::normalize(key).ok_or(SessionError::InvalidApiKey)?;
    let verification = provider_key_check::verify(engine, found, key).await;
    if verification == Verification::Verified {
        keychain::save(&name, key)?;
    }
    Ok(ProviderKeyResult {
        verification,
        last_four: keychain::last_four(&name)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agent_run::ErrorKind;
    use crate::services::agent_run_engine::testing::{Harness, chat, fast_limits, harness};
    use crate::services::availability_check::{self, Availability};
    use crate::services::{provider_keys_list, workspace_create};
    use serde_json::json;
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn server(response: ResponseTemplate) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(response)
            .mount(&server)
            .await;
        server
    }

    async fn stored(h: &Harness, provider: &str) -> (String, Option<String>) {
        let (_, name) = provider_key_check::target(&h.engine, provider)
            .await
            .unwrap();
        let secret = keychain::read(&name).unwrap();
        (name, secret)
    }

    #[tokio::test]
    async fn verifies_with_a_minimal_request_to_the_catalog_url_and_saves_the_trimmed_key() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer sk-groq-new-1234"))
            .and(body_partial_json(json!({
                "model": "openai/gpt-oss-120b",
                "max_completion_tokens": 16,
            })))
            .respond_with(chat(&["OK"]))
            .expect(1)
            .mount(&server)
            .await;
        let h = harness(&server.uri(), fast_limits(), &[]).await;

        let saved = run(&h.engine, "groq", "  sk-groq-new-1234\n")
            .await
            .unwrap();
        let (name, secret) = stored(&h, "groq").await;
        keychain::remove(&name).unwrap();

        assert_eq!(
            saved,
            ProviderKeyResult {
                verification: Verification::Verified,
                last_four: Some("1234".into()),
            }
        );
        assert_eq!(secret.as_deref(), Some("sk-groq-new-1234"));
    }

    #[tokio::test]
    async fn the_result_sent_to_the_ui_never_contains_the_key() {
        for response in [chat(&["OK"]), ResponseTemplate::new(401)] {
            let server = server(response).await;
            let h = harness(&server.uri(), fast_limits(), &[]).await;

            let saved = run(&h.engine, "groq", "sk-secret-value-7777")
                .await
                .unwrap();
            let (name, _) = stored(&h, "groq").await;
            keychain::remove(&name).unwrap();

            let json = serde_json::to_string(&saved).unwrap();
            assert!(!json.contains("sk-secret-value"), "{json}");
        }
    }

    #[tokio::test]
    async fn a_key_the_provider_rejects_is_not_saved() {
        for (status, reason) in [
            (401, ErrorKind::ProviderAuthFailed),
            (402, ErrorKind::ProviderBilling),
            (404, ErrorKind::ModelUnavailable),
            (400, ErrorKind::ProviderRejected),
        ] {
            let server = server(ResponseTemplate::new(status)).await;
            let h = harness(&server.uri(), fast_limits(), &[]).await;

            let saved = run(&h.engine, "openai", "sk-bad-0000").await.unwrap();

            assert_eq!(
                saved,
                ProviderKeyResult {
                    verification: Verification::Rejected { reason },
                    last_four: None,
                },
                "{status}"
            );
            assert_eq!(stored(&h, "openai").await.1, None);
        }
    }

    #[tokio::test]
    async fn a_key_that_cannot_be_verified_is_not_saved() {
        for status in [503, 429] {
            let server = server(ResponseTemplate::new(status)).await;
            let h = harness(&server.uri(), fast_limits(), &[]).await;

            let saved = run(&h.engine, "anthropic", "sk-ant-1111").await.unwrap();

            assert_eq!(saved.verification, Verification::ProviderBusy, "{status}");
            assert_eq!(stored(&h, "anthropic").await.1, None);
        }
    }

    #[tokio::test]
    async fn without_a_connection_the_key_is_not_saved_and_the_provider_is_marked_offline() {
        let h = harness("http://127.0.0.1:1", fast_limits(), &[]).await;

        let saved = run(&h.engine, "groq", "sk-groq-2222").await.unwrap();

        assert_eq!(saved.verification, Verification::Offline);
        assert_eq!(stored(&h, "groq").await.1, None);
        assert!(h.engine.registry.is_unreachable("groq"));
    }

    #[tokio::test]
    async fn a_failed_replacement_keeps_the_previous_key() {
        let server = server(ResponseTemplate::new(401)).await;
        let h = harness(&server.uri(), fast_limits(), &[]).await;
        let (name, _) = stored(&h, "groq").await;
        keychain::save(&name, "sk-old-valid-4321").unwrap();

        let saved = run(&h.engine, "groq", "sk-new-wrong-8888").await.unwrap();
        let kept = keychain::read(&name).unwrap();
        keychain::remove(&name).unwrap();

        assert_eq!(saved.last_four.as_deref(), Some("4321"));
        assert_eq!(kept.as_deref(), Some("sk-old-valid-4321"));
    }

    #[tokio::test]
    async fn a_verified_replacement_overwrites_the_previous_key() {
        let server = server(chat(&["OK"])).await;
        let h = harness(&server.uri(), fast_limits(), &[]).await;
        let (name, _) = stored(&h, "groq").await;
        keychain::save(&name, "sk-old-valid-4321").unwrap();

        let saved = run(&h.engine, "groq", "sk-new-valid-6789").await.unwrap();
        let replaced = keychain::read(&name).unwrap();
        keychain::remove(&name).unwrap();

        assert_eq!(saved.last_four.as_deref(), Some("6789"));
        assert_eq!(replaced.as_deref(), Some("sk-new-valid-6789"));
    }

    #[tokio::test]
    async fn invalid_keys_and_providers_fail_before_any_request() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(chat(&["OK"]))
            .expect(0)
            .mount(&server)
            .await;
        let h = harness(&server.uri(), fast_limits(), &[]).await;

        for key in ["", "   ", "sk with spaces"] {
            assert!(matches!(
                run(&h.engine, "groq", key).await,
                Err(SessionError::InvalidApiKey)
            ));
        }
        assert!(matches!(
            run(&h.engine, "ollama", "sk-1234").await,
            Err(SessionError::InvalidProvider)
        ));
    }

    #[tokio::test]
    async fn the_saved_key_belongs_to_the_current_workspace_and_is_used_by_its_agents() {
        let server = server(chat(&["OK"])).await;
        let h = harness(
            &server.uri(),
            fast_limits(),
            &[("groq", "openai/gpt-oss-120b")],
        )
        .await;
        run(&h.engine, "groq", "sk-acme-3333").await.unwrap();
        let (name, _) = stored(&h, "groq").await;
        let availability = availability_check::run(&h.engine, &h.room_id)
            .await
            .unwrap();

        workspace_create::run(&h.engine.pool, "Globex")
            .await
            .unwrap();
        let other = provider_keys_list::run(&h.engine).await.unwrap();
        let still_there = keychain::read(&name).unwrap();
        keychain::remove(&name).unwrap();

        assert_eq!(availability[0].availability, Availability::Available);
        assert!(other.iter().all(|status| status.last_four.is_none()));
        assert_eq!(still_there.as_deref(), Some("sk-acme-3333"));
    }
}
