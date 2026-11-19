use crate::services::agent_run_engine::Engine;
use crate::services::provider_key_check;
use crate::services::provider_keys_list::{self, ProviderKeyStatus};
use crate::services::session_load::SessionError;
use crate::system::keychain;

pub async fn run(engine: &Engine, provider: &str) -> Result<Vec<ProviderKeyStatus>, SessionError> {
    let (_, name) = provider_key_check::target(engine, provider).await?;
    keychain::remove(&name)?;
    provider_keys_list::run(engine).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::agent_run_engine::testing::{fast_limits, harness};
    use crate::services::availability_check::{self, Availability};
    use crate::services::{workspace_create, workspace_switch};

    #[tokio::test]
    async fn deletes_the_key_idempotently_and_its_agents_need_setup_again() {
        let h = harness(
            "http://127.0.0.1:1",
            fast_limits(),
            &[("groq", "openai/gpt-oss-120b")],
        )
        .await;
        let (_, name) = provider_key_check::target(&h.engine, "groq").await.unwrap();
        keychain::save(&name, "sk-groq-1357").unwrap();

        let listed = run(&h.engine, "groq").await.unwrap();
        let again = run(&h.engine, "groq").await;
        let availability = availability_check::run(&h.engine, &h.room_id)
            .await
            .unwrap();

        assert_eq!(keychain::read(&name).unwrap(), None);
        assert!(listed.iter().all(|status| status.last_four.is_none()));
        assert!(again.is_ok());
        assert_eq!(availability[0].availability, Availability::NeedsSetup);
    }

    #[tokio::test]
    async fn deleting_in_one_workspace_keeps_the_key_of_another() {
        let h = harness("http://127.0.0.1:1", fast_limits(), &[]).await;
        let (_, acme) = provider_key_check::target(&h.engine, "anthropic")
            .await
            .unwrap();
        keychain::save(&acme, "sk-ant-acme-1111").unwrap();
        let acme_id = acme.split('/').next().unwrap().to_string();
        workspace_create::run(&h.engine.pool, "Globex")
            .await
            .unwrap();

        run(&h.engine, "anthropic").await.unwrap();
        workspace_switch::run(&h.engine.pool, &acme_id)
            .await
            .unwrap();
        let listed = provider_keys_list::run(&h.engine).await.unwrap();
        keychain::remove(&acme).unwrap();

        assert_eq!(listed[0].last_four.as_deref(), Some("1111"));
    }

    #[tokio::test]
    async fn local_providers_have_no_key_to_delete() {
        let h = harness("http://127.0.0.1:1", fast_limits(), &[]).await;

        assert!(matches!(
            run(&h.engine, "ollama").await,
            Err(SessionError::InvalidProvider)
        ));
    }
}
