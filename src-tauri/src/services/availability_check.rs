use crate::catalogs::llm::{Location, Provider};
use crate::domain::agent_run::ErrorKind;
use crate::repositories::{agents, room_agents, rooms};
use crate::services::agent_run_engine::Engine;
use crate::services::session_load::{self, SessionError};
use crate::system::{keychain, local_models};
use futures_util::future::join_all;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Available,
    NeedsSetup,
    Unavailable,
    NeedsConnection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentAvailability {
    pub agent_id: String,
    pub provider: String,
    pub availability: Availability,
    pub reason: Option<ErrorKind>,
}

pub async fn run(engine: &Engine, room_id: &str) -> Result<Vec<AgentAvailability>, SessionError> {
    let workspace_id = session_load::require(&engine.pool)
        .await?
        .current_workspace_id;
    let mut transaction = engine.pool.begin().await?;
    if rooms::get(&mut *transaction, &workspace_id, room_id)
        .await?
        .is_none()
    {
        return Err(SessionError::RoomNotFound);
    }
    let mut targets = Vec::new();
    for (room, participant) in room_agents::list(&mut *transaction, &workspace_id).await? {
        if room != room_id {
            continue;
        }
        if let Some(agent) = agents::get(&mut *transaction, &workspace_id, &participant.id).await? {
            targets.push((agent.id, agent.provider, agent.model));
        }
    }
    transaction.commit().await?;

    let providers: HashMap<&str, &Provider> = engine
        .providers
        .iter()
        .map(|provider| (provider.key.as_str(), provider))
        .collect();
    let mut local_keys: Vec<&str> = targets
        .iter()
        .filter(|(_, key, _)| {
            providers
                .get(key.as_str())
                .is_some_and(|provider| provider.location == Location::Local)
        })
        .map(|(_, key, _)| key.as_str())
        .collect();
    local_keys.sort_unstable();
    local_keys.dedup();
    let detected: HashMap<&str, Option<Vec<String>>> =
        join_all(local_keys.into_iter().map(|key| {
            let base_url = providers[key].base_url.clone();
            async move { (key, local_models::list(&base_url).await.ok()) }
        }))
        .await
        .into_iter()
        .collect();

    Ok(targets
        .iter()
        .map(|(agent_id, key, model)| {
            let (availability, reason) = match providers.get(key.as_str()) {
                None => (Availability::Unavailable, Some(ErrorKind::ModelUnavailable)),
                Some(provider) if provider.location == Location::Local => {
                    local(detected.get(key.as_str()).and_then(Option::as_ref), model)
                }
                Some(provider) => cloud(engine, &workspace_id, provider, model),
            };
            AgentAvailability {
                agent_id: agent_id.clone(),
                provider: key.clone(),
                availability,
                reason,
            }
        })
        .collect())
}

fn local(models: Option<&Vec<String>>, model: &str) -> (Availability, Option<ErrorKind>) {
    match models {
        None => (
            Availability::Unavailable,
            Some(ErrorKind::ProviderUnavailable),
        ),
        Some(models) if !models.iter().any(|entry| entry == model) => {
            (Availability::Unavailable, Some(ErrorKind::ModelUnavailable))
        }
        Some(_) => (Availability::Available, None),
    }
}

fn cloud(
    engine: &Engine,
    workspace_id: &str,
    provider: &Provider,
    model: &str,
) -> (Availability, Option<ErrorKind>) {
    if !provider.models.iter().any(|entry| entry.key == model) {
        return (Availability::Unavailable, Some(ErrorKind::ModelUnavailable));
    }
    let has_key = keychain::read(&format!("{workspace_id}/{}", provider.key))
        .ok()
        .flatten()
        .is_some();
    if !has_key {
        return (
            Availability::NeedsSetup,
            Some(ErrorKind::ProviderNotConfigured),
        );
    }
    if engine.registry.is_unreachable(&provider.key) {
        return (
            Availability::NeedsConnection,
            Some(ErrorKind::ProviderUnavailable),
        );
    }
    (Availability::Available, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::recipient::Selection;
    use crate::services::agent_run_engine::testing::{chat, fast_limits, harness};
    use crate::services::intervention_send::{self, Intervention, InterventionSent};
    use crate::services::workspace_create;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn local_server(models: &[&str]) -> MockServer {
        let server = MockServer::start().await;
        let data: Vec<_> = models
            .iter()
            .map(|id| serde_json::json!({ "id": id }))
            .collect();
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "data": data })),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .respond_with(chat(&["ok"]))
            .mount(&server)
            .await;
        server
    }

    fn states(found: &[AgentAvailability]) -> Vec<(Availability, Option<ErrorKind>)> {
        found
            .iter()
            .map(|entry| (entry.availability, entry.reason))
            .collect()
    }

    async fn workspace_secret(
        h: &crate::services::agent_run_engine::testing::Harness,
        provider: &str,
    ) -> String {
        let workspace_id = session_load::require(&h.engine.pool)
            .await
            .unwrap()
            .current_workspace_id;
        format!("{workspace_id}/{provider}")
    }

    #[tokio::test]
    async fn local_agents_are_available_only_with_their_server_running_and_their_model_installed() {
        let server = local_server(&["qwen-test"]).await;
        let h = harness(
            &server.uri(),
            fast_limits(),
            &[("ollama", "qwen-test"), ("lmstudio", "missing-model")],
        )
        .await;

        let found = run(&h.engine, &h.room_id).await.unwrap();

        assert_eq!(found[0].agent_id, h.agents[0]);
        assert_eq!(found[0].provider, "ollama");
        assert_eq!(
            states(&found),
            [
                (Availability::Available, None),
                (Availability::Unavailable, Some(ErrorKind::ModelUnavailable)),
            ]
        );
    }

    #[tokio::test]
    async fn a_local_server_that_is_not_running_makes_its_agents_unavailable() {
        let h = harness(
            "http://127.0.0.1:1",
            fast_limits(),
            &[("ollama", "qwen-test")],
        )
        .await;

        let found = run(&h.engine, &h.room_id).await.unwrap();

        assert_eq!(
            states(&found),
            [(
                Availability::Unavailable,
                Some(ErrorKind::ProviderUnavailable)
            )]
        );
    }

    #[tokio::test]
    async fn cloud_agents_need_a_key_a_catalog_model_and_their_last_connection() {
        let server = local_server(&[]).await;
        let h = harness(
            &server.uri(),
            fast_limits(),
            &[("groq", "openai/gpt-oss-120b"), ("ollama", "qwen-test")],
        )
        .await;
        assert_eq!(
            states(&run(&h.engine, &h.room_id).await.unwrap())[0],
            (
                Availability::NeedsSetup,
                Some(ErrorKind::ProviderNotConfigured)
            )
        );
        let secret = workspace_secret(&h, "groq").await;
        keychain::save(&secret, "sk-groq-test").unwrap();

        let configured = states(&run(&h.engine, &h.room_id).await.unwrap())[0];
        h.engine.registry.reached("groq", false);
        let offline = states(&run(&h.engine, &h.room_id).await.unwrap())[0];
        sqlx::query("UPDATE agents SET model = 'retired' WHERE id = ?")
            .bind(&h.agents[0])
            .execute(&h.engine.pool)
            .await
            .unwrap();
        let retired = states(&run(&h.engine, &h.room_id).await.unwrap())[0];
        keychain::remove(&secret).unwrap();

        assert_eq!(configured, (Availability::Available, None));
        assert_eq!(
            offline,
            (
                Availability::NeedsConnection,
                Some(ErrorKind::ProviderUnavailable)
            )
        );
        assert_eq!(
            retired,
            (Availability::Unavailable, Some(ErrorKind::ModelUnavailable))
        );
    }

    #[tokio::test]
    async fn a_failed_connection_is_remembered_until_the_provider_answers_again() {
        let h = harness(
            "http://127.0.0.1:1",
            fast_limits(),
            &[("groq", "openai/gpt-oss-120b")],
        )
        .await;
        let secret = workspace_secret(&h, "groq").await;
        keychain::save(&secret, "sk-groq-test").unwrap();
        let selection = Selection::Agent(h.agents[0].clone());
        let sent = intervention_send::run(
            &h.engine,
            Intervention {
                room_id: &h.room_id,
                body: "Hi",
                selection: &selection,
                mention_ids: &[],
                client_request_id: &uuid::Uuid::now_v7().to_string(),
            },
        )
        .await
        .unwrap();
        let InterventionSent::Sent {
            run: Some(started), ..
        } = sent
        else {
            panic!("expected a run");
        };
        h.finished(&started.id).await;

        let offline = states(&run(&h.engine, &h.room_id).await.unwrap())[0];
        keychain::remove(&secret).unwrap();

        assert_eq!(
            offline,
            (
                Availability::NeedsConnection,
                Some(ErrorKind::ProviderUnavailable)
            )
        );
    }

    #[tokio::test]
    async fn a_successful_reply_clears_the_connection_mark() {
        let server = local_server(&["qwen-test"]).await;
        let h = harness(&server.uri(), fast_limits(), &[("ollama", "qwen-test")]).await;
        h.engine.registry.reached("ollama", false);

        let sent = h.send(0, "Hi").await.unwrap();
        h.finished(&sent.run.id).await;

        assert!(!h.engine.registry.is_unreachable("ollama"));
    }

    #[tokio::test]
    async fn only_rooms_of_the_current_workspace_are_checked() {
        let server = local_server(&["qwen-test"]).await;
        let h = harness(&server.uri(), fast_limits(), &[("ollama", "qwen-test")]).await;
        workspace_create::run(&h.engine.pool, "Globex")
            .await
            .unwrap();

        assert!(matches!(
            run(&h.engine, &h.room_id).await,
            Err(SessionError::RoomNotFound)
        ));
    }
}
