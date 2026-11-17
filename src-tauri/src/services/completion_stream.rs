use crate::system::chat_completions::{self, ChatStreamError};
use crate::system::keychain::{self, KeychainError};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum CompletionStreamError {
    #[error(transparent)]
    Keychain(#[from] KeychainError),
    #[error("secret not found")]
    MissingSecret,
    #[error(transparent)]
    Stream(#[from] ChatStreamError),
}

impl Serialize for CompletionStreamError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub async fn run(
    base_url: &str,
    model: &str,
    prompt: &str,
    secret_name: Option<&str>,
    on_token: impl Fn(String),
) -> Result<(), CompletionStreamError> {
    let api_key = match secret_name {
        Some(name) => Some(keychain::read(name)?.ok_or(CompletionStreamError::MissingSecret)?),
        None => None,
    };
    chat_completions::stream(base_url, api_key.as_deref(), model, prompt, on_token).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use wiremock::matchers::{header, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sse_ok() -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_raw(
            "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\ndata: [DONE]\n\n",
            "text/event-stream",
        )
    }

    #[tokio::test]
    async fn streams_without_secret() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(sse_ok())
            .mount(&server)
            .await;
        let tokens = RefCell::new(Vec::new());

        run(&server.uri(), "m", "p", None, |token| {
            tokens.borrow_mut().push(token)
        })
        .await
        .unwrap();

        assert_eq!(tokens.into_inner(), ["hi"]);
    }

    #[tokio::test]
    async fn uses_secret_from_keychain() {
        let name = "test/completion-stream-key";
        keychain::save(name, "sk-from-keychain").unwrap();
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(header("authorization", "Bearer sk-from-keychain"))
            .respond_with(sse_ok())
            .expect(1)
            .mount(&server)
            .await;

        let result = run(&server.uri(), "m", "p", Some(name), |_| {}).await;

        keychain::remove(name).unwrap();
        result.unwrap();
    }

    #[tokio::test]
    async fn fails_when_secret_is_missing() {
        let result = run(
            "http://127.0.0.1:1",
            "m",
            "p",
            Some("test/completion-stream-missing"),
            |_| {},
        )
        .await;

        assert!(matches!(result, Err(CompletionStreamError::MissingSecret)));
    }
}
