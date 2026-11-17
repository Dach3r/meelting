use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const STREAM_END: &str = "[DONE]";

#[derive(Debug, thiserror::Error)]
pub enum ChatStreamError {
    #[error("request failed")]
    Request(#[from] reqwest::Error),
    #[error("provider responded with status {0}")]
    Status(u16),
    #[error("invalid response")]
    InvalidResponse,
}

#[derive(Deserialize)]
struct Chunk {
    #[serde(default)]
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    delta: Delta,
}

#[derive(Deserialize)]
struct Delta {
    content: Option<String>,
}

pub async fn stream(
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
    prompt: &str,
    on_token: impl Fn(String),
) -> Result<(), ChatStreamError> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let mut request = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .build()?
        .post(url)
        .json(&json!({
            "model": model,
            "messages": [{ "role": "user", "content": prompt }],
            "stream": true,
        }));
    if let Some(key) = api_key {
        request = request.bearer_auth(key);
    }
    let response = request.send().await?;
    if !response.status().is_success() {
        return Err(ChatStreamError::Status(response.status().as_u16()));
    }

    let mut events = response.bytes_stream().eventsource();
    while let Some(event) = events.next().await {
        let event = event.map_err(|_| ChatStreamError::InvalidResponse)?;
        if event.data == STREAM_END {
            return Ok(());
        }
        let chunk: Chunk =
            serde_json::from_str(&event.data).map_err(|_| ChatStreamError::InvalidResponse)?;
        for token in chunk
            .choices
            .into_iter()
            .filter_map(|choice| choice.delta.content)
        {
            on_token(token);
        }
    }
    Err(ChatStreamError::InvalidResponse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sse(events: &[&str]) -> ResponseTemplate {
        let body: String = events
            .iter()
            .map(|event| format!("data: {event}\n\n"))
            .collect();
        ResponseTemplate::new(200).set_body_raw(body, "text/event-stream")
    }

    async fn collect(
        server: &MockServer,
        api_key: Option<&str>,
    ) -> (Vec<String>, Result<(), ChatStreamError>) {
        let tokens = RefCell::new(Vec::new());
        let result = stream(&server.uri(), api_key, "test-model", "hi", |token| {
            tokens.borrow_mut().push(token)
        })
        .await;
        (tokens.into_inner(), result)
    }

    #[tokio::test]
    async fn streams_tokens_in_order() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_partial_json(json!({
                "model": "test-model",
                "stream": true,
                "messages": [{ "role": "user", "content": "hi" }],
            })))
            .respond_with(sse(&[
                r#"{"choices":[{"delta":{"role":"assistant","content":""}}]}"#,
                r#"{"choices":[{"delta":{"content":"Hel"}}]}"#,
                r#"{"choices":[{"delta":{"content":"lo"}}]}"#,
                r#"{"choices":[]}"#,
                "[DONE]",
            ]))
            .mount(&server)
            .await;

        let (tokens, result) = collect(&server, None).await;

        result.unwrap();
        assert_eq!(tokens.concat(), "Hello");
    }

    #[tokio::test]
    async fn sends_api_key_as_bearer_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(header("authorization", "Bearer sk-test"))
            .respond_with(sse(&[
                r#"{"choices":[{"delta":{"content":"ok"}}]}"#,
                "[DONE]",
            ]))
            .expect(1)
            .mount(&server)
            .await;

        let (tokens, result) = collect(&server, Some("sk-test")).await;

        result.unwrap();
        assert_eq!(tokens, ["ok"]);
    }

    #[tokio::test]
    async fn reports_status_without_provider_text() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(401).set_body_string("secret detail"))
            .mount(&server)
            .await;

        let (tokens, result) = collect(&server, None).await;

        assert!(tokens.is_empty());
        let error = result.unwrap_err();
        assert!(matches!(error, ChatStreamError::Status(401)));
        assert!(!error.to_string().contains("secret detail"));
    }

    #[tokio::test]
    async fn rejects_malformed_events() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(sse(&["not json"]))
            .mount(&server)
            .await;

        let (_, result) = collect(&server, None).await;

        assert!(matches!(result, Err(ChatStreamError::InvalidResponse)));
    }

    #[tokio::test]
    async fn rejects_stream_that_ends_without_done() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(sse(&[r#"{"choices":[{"delta":{"content":"cut"}}]}"#]))
            .mount(&server)
            .await;

        let (tokens, result) = collect(&server, None).await;

        assert_eq!(tokens, ["cut"]);
        assert!(matches!(result, Err(ChatStreamError::InvalidResponse)));
    }

    #[tokio::test]
    #[ignore = "needs MEELTING_LOCAL_BASE_URL and MEELTING_LOCAL_MODEL pointing at a running local provider"]
    async fn streams_from_real_local_provider() {
        let base_url = std::env::var("MEELTING_LOCAL_BASE_URL").unwrap();
        let model = std::env::var("MEELTING_LOCAL_MODEL").unwrap();
        let tokens = RefCell::new(Vec::new());

        stream(
            &base_url,
            None,
            &model,
            "Reply with the single word: ok",
            |token| tokens.borrow_mut().push(token),
        )
        .await
        .unwrap();

        assert!(!tokens.into_inner().concat().is_empty());
    }
}
