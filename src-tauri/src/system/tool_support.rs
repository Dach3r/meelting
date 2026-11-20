use crate::catalogs::llm::{Location, Provider};
use crate::domain::tool_support::{ToolSupport, from_lmstudio_models, from_ollama_show};
use reqwest::{Client, Response, redirect::Policy};
use serde_json::{Value, json};
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
const TIMEOUT: Duration = Duration::from_secs(3);
const MAX_BODY_BYTES: usize = 1024 * 1024;

pub async fn detect(provider: &Provider, model: &str) -> ToolSupport {
    if provider.location != Location::Local {
        return ToolSupport::Unknown;
    }
    let root = provider.base_url.trim_end_matches('/');
    let root = root.strip_suffix("/v1").unwrap_or(root);
    let Ok(client) = Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(TIMEOUT)
        .redirect(Policy::none())
        .build()
    else {
        return ToolSupport::Unknown;
    };
    match provider.key.as_str() {
        "ollama" => {
            let request = client
                .post(format!("{root}/api/show"))
                .json(&json!({ "model": model }));
            match body(request).await {
                Some(body) => from_ollama_show(&body),
                None => ToolSupport::Unknown,
            }
        }
        "lmstudio" => match body(client.get(format!("{root}/api/v1/models"))).await {
            Some(body) => from_lmstudio_models(&body, model),
            None => ToolSupport::Unknown,
        },
        _ => ToolSupport::Unknown,
    }
}

async fn body(request: reqwest::RequestBuilder) -> Option<Value> {
    let mut response: Response = request.send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if bytes.len() + chunk.len() > MAX_BODY_BYTES {
            return None;
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogs::llm::{Protocol, Provider};
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn provider(key: &str, base_url: String) -> Provider {
        Provider {
            key: key.into(),
            name: key.into(),
            location: Location::Local,
            protocol: Protocol::ChatCompletions,
            base_url,
            output_tokens_param: None,
            models: vec![],
        }
    }

    async fn ollama(response: ResponseTemplate) -> ToolSupport {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/show"))
            .and(body_json(json!({ "model": "qwen3:4b" })))
            .respond_with(response)
            .mount(&server)
            .await;
        detect(
            &provider("ollama", format!("{}/v1", server.uri())),
            "qwen3:4b",
        )
        .await
    }

    async fn lm_studio(response: ResponseTemplate) -> ToolSupport {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/models"))
            .respond_with(response)
            .mount(&server)
            .await;
        detect(
            &provider("lmstudio", format!("{}/v1/", server.uri())),
            "qwen/qwen3-4b",
        )
        .await
    }

    #[tokio::test]
    async fn ollama_is_asked_for_the_capabilities_of_exactly_that_model() {
        let with = ResponseTemplate::new(200)
            .set_body_json(json!({"capabilities": ["completion", "tools"]}));
        let without =
            ResponseTemplate::new(200).set_body_json(json!({"capabilities": ["completion"]}));

        assert_eq!(ollama(with).await, ToolSupport::Supported);
        assert_eq!(ollama(without).await, ToolSupport::Unsupported);
    }

    #[tokio::test]
    async fn ollama_failures_are_unknown_never_unsupported() {
        let cases = [
            ResponseTemplate::new(404)
                .set_body_json(json!({"error": "model 'qwen3:4b' not found"})),
            ResponseTemplate::new(500),
            ResponseTemplate::new(405),
            ResponseTemplate::new(200).set_body_string("not json"),
            ResponseTemplate::new(200).set_body_json(json!({"details": {}})),
            ResponseTemplate::new(200).set_body_json(json!({"capabilities": []})),
            ResponseTemplate::new(302).insert_header("location", "http://127.0.0.1:1/api/show"),
            ResponseTemplate::new(200)
                .set_body_json(json!({"capabilities": ["tools"]}))
                .set_delay(Duration::from_secs(5)),
            ResponseTemplate::new(200).set_body_bytes(vec![b' '; MAX_BODY_BYTES + 1]),
        ];
        for response in cases {
            assert_eq!(ollama(response).await, ToolSupport::Unknown);
        }
    }

    #[tokio::test]
    async fn lm_studio_uses_its_native_v1_models_endpoint() {
        let body = |flag: Value| {
            ResponseTemplate::new(200).set_body_json(json!({"models": [{
                "type": "llm", "key": "qwen/qwen3-4b",
                "capabilities": {"vision": false, "trained_for_tool_use": flag}
            }]}))
        };

        assert_eq!(lm_studio(body(json!(true))).await, ToolSupport::Supported);
        assert_eq!(
            lm_studio(body(json!(false))).await,
            ToolSupport::Unsupported
        );
        assert_eq!(lm_studio(body(json!(null))).await, ToolSupport::Unknown);
    }

    #[tokio::test]
    async fn an_lm_studio_without_the_v1_api_or_with_a_bad_answer_is_unknown() {
        for response in [
            ResponseTemplate::new(404),
            ResponseTemplate::new(200).set_body_string("<html>"),
            ResponseTemplate::new(200).set_body_json(json!({"data": [{"id": "qwen/qwen3-4b"}]})),
        ] {
            assert_eq!(lm_studio(response).await, ToolSupport::Unknown);
        }
    }

    #[tokio::test]
    async fn an_unreachable_runtime_or_an_unknown_provider_is_unknown_without_blocking() {
        let started = std::time::Instant::now();

        assert_eq!(
            detect(&provider("ollama", "http://127.0.0.1:1/v1".into()), "m").await,
            ToolSupport::Unknown
        );
        assert_eq!(
            detect(
                &provider("other-local", "http://127.0.0.1:1/v1".into()),
                "m"
            )
            .await,
            ToolSupport::Unknown
        );
        let mut cloud = provider("ollama", "http://127.0.0.1:1/v1".into());
        cloud.location = Location::Cloud;
        assert_eq!(detect(&cloud, "m").await, ToolSupport::Unknown);
        assert!(started.elapsed() < Duration::from_secs(4));
    }

    #[tokio::test]
    async fn only_the_configured_endpoint_is_contacted() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"capabilities": ["tools"]})),
            )
            .expect(1)
            .mount(&server)
            .await;

        detect(
            &provider("ollama", format!("{}/v1", server.uri())),
            "qwen3:4b",
        )
        .await;

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url.path(), "/api/show");
    }

    #[tokio::test]
    #[ignore = "needs a running local runtime: MEELTING_LOCAL_PROVIDER (ollama or lmstudio), MEELTING_LOCAL_BASE_URL and MEELTING_LOCAL_MODEL"]
    async fn reads_the_capability_from_a_real_local_runtime() {
        let key = std::env::var("MEELTING_LOCAL_PROVIDER").unwrap();
        let base_url = std::env::var("MEELTING_LOCAL_BASE_URL").unwrap();
        let model = std::env::var("MEELTING_LOCAL_MODEL").unwrap();

        let support = detect(&provider(&key, base_url), &model).await;

        println!("{key} {model}: {support:?}");
    }
}
