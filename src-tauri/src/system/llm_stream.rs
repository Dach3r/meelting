use crate::domain::agent_run::ErrorKind;
use crate::domain::prompt::Prompt;
use crate::domain::provider_error;
use eventsource_stream::{Event, EventStreamError, Eventsource};
use futures_util::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failure {
    pub kind: ErrorKind,
    pub retry_after: Option<Duration>,
    pub unreachable: bool,
}

impl From<ErrorKind> for Failure {
    fn from(kind: ErrorKind) -> Self {
        Failure {
            kind,
            retry_after: None,
            unreachable: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
}

pub struct StreamRequest<'a> {
    pub base_url: &'a str,
    pub api_key: Option<&'a str>,
    pub model: &'a str,
    pub max_output_tokens: u32,
    pub prompt: &'a Prompt,
    pub connect_timeout: Duration,
    pub idle_timeout: Duration,
}

pub type OnText<'a> = dyn FnMut(&str) -> Result<(), Failure> + Send + 'a;

type EventItem = Result<Event, EventStreamError<reqwest::Error>>;

pub struct Events {
    inner: Pin<Box<dyn Stream<Item = EventItem> + Send>>,
    idle: Duration,
}

pub async fn open(
    request: &StreamRequest<'_>,
    path: &str,
    headers: &[(&str, &str)],
    body: Value,
) -> Result<Events, Failure> {
    let client = reqwest::Client::builder()
        .connect_timeout(request.connect_timeout)
        .build()
        .map_err(|_| Failure::from(ErrorKind::ProviderUnavailable))?;
    let url = format!("{}{path}", request.base_url.trim_end_matches('/'));
    let mut builder = client.post(url).json(&body);
    if let Some(key) = request.api_key {
        builder = builder.bearer_auth(key);
    }
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let response = tokio::time::timeout(request.idle_timeout, builder.send())
        .await
        .map_err(|_| Failure::from(ErrorKind::Timeout))?
        .map_err(|error| Failure {
            unreachable: error.is_connect(),
            ..Failure::from(ErrorKind::ProviderUnavailable)
        })?;
    let status = response.status();
    if !status.is_success() {
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(provider_error::retry_after);
        let body = tokio::time::timeout(request.idle_timeout, response.bytes())
            .await
            .ok()
            .and_then(Result::ok)
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
        let code = body.as_ref().and_then(error_code);
        return Err(Failure {
            kind: provider_error::from_status(status.as_u16(), code.as_deref()),
            retry_after,
            unreachable: false,
        });
    }
    Ok(Events {
        inner: Box::pin(response.bytes_stream().eventsource()),
        idle: request.idle_timeout,
    })
}

impl Events {
    pub async fn next(&mut self) -> Result<Option<Event>, Failure> {
        match tokio::time::timeout(self.idle, self.inner.next()).await {
            Err(_) => Err(ErrorKind::Timeout.into()),
            Ok(None) => Ok(None),
            Ok(Some(Err(_))) => Err(ErrorKind::ProviderUnavailable.into()),
            Ok(Some(Ok(event))) => Ok(Some(event)),
        }
    }
}

pub fn parse(data: &str) -> Result<Value, Failure> {
    serde_json::from_str(data).map_err(|_| ErrorKind::InvalidResponse.into())
}

pub fn error_code(value: &Value) -> Option<String> {
    let error = value.get("error").unwrap_or(value);
    ["code", "type"]
        .iter()
        .find_map(|field| error.get(*field).and_then(Value::as_str))
        .map(str::to_string)
}

pub fn stream_error(value: &Value) -> Failure {
    error_code(value)
        .map_or(ErrorKind::ProviderUnavailable, |code| {
            provider_error::from_code(&code)
        })
        .into()
}

pub fn tokens(value: &Value, field: &str) -> Option<i64> {
    value.get(field).and_then(Value::as_i64)
}

#[cfg(test)]
pub mod testing {
    use super::*;
    use crate::domain::prompt::{Role, Turn};
    use wiremock::ResponseTemplate;

    pub fn prompt() -> Prompt {
        Prompt {
            system: "Be brief.".into(),
            turns: vec![
                Turn {
                    role: Role::User,
                    text: "Hi".into(),
                },
                Turn {
                    role: Role::Assistant,
                    text: "Hello".into(),
                },
                Turn {
                    role: Role::User,
                    text: "Status?".into(),
                },
            ],
        }
    }

    pub fn request<'a>(base_url: &'a str, prompt: &'a Prompt) -> StreamRequest<'a> {
        StreamRequest {
            base_url,
            api_key: Some("sk-test"),
            model: "test-model",
            max_output_tokens: 256,
            prompt,
            connect_timeout: Duration::from_secs(2),
            idle_timeout: Duration::from_millis(500),
        }
    }

    pub fn sse(events: &[&str]) -> ResponseTemplate {
        let body: String = events
            .iter()
            .map(|event| format!("data: {event}\n\n"))
            .collect();
        ResponseTemplate::new(200).set_body_raw(body, "text/event-stream")
    }

    pub fn collector(
        texts: &mut Vec<String>,
    ) -> impl FnMut(&str) -> Result<(), Failure> + Send + '_ {
        move |text| {
            texts.push(text.to_string());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn open_against(response: ResponseTemplate) -> Result<Events, Failure> {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(response)
            .mount(&server)
            .await;
        let prompt = prompt();
        let uri = server.uri();
        open(&request(&uri, &prompt), "/x", &[], Value::Null).await
    }

    #[tokio::test]
    async fn classifies_http_errors_by_status_and_code_with_retry_after() {
        let rate_limited = open_against(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "3")
                .set_body_string("{\"error\":{\"message\":\"slow down\"}}"),
        )
        .await;
        let quota = open_against(
            ResponseTemplate::new(429)
                .set_body_string("{\"error\":{\"code\":\"insufficient_quota\"}}"),
        )
        .await;
        let overloaded = open_against(ResponseTemplate::new(529)).await;

        assert_eq!(
            rate_limited.err(),
            Some(Failure {
                kind: ErrorKind::RateLimited,
                retry_after: Some(Duration::from_secs(3)),
                unreachable: false,
            })
        );
        assert_eq!(
            quota.err().map(|f| f.kind),
            Some(ErrorKind::ProviderBilling)
        );
        assert_eq!(
            overloaded.err().map(|f| f.kind),
            Some(ErrorKind::ProviderUnavailable)
        );
    }

    #[tokio::test]
    async fn a_refused_connection_is_an_unreachable_provider() {
        let prompt = prompt();
        let mut refused = request("http://127.0.0.1:1", &prompt);
        refused.connect_timeout = Duration::from_secs(5);
        refused.idle_timeout = Duration::from_secs(10);
        let result = open(&refused, "/x", &[], Value::Null).await;

        assert_eq!(
            result.err().map(|f| (f.kind, f.unreachable)),
            Some((ErrorKind::ProviderUnavailable, true))
        );
        assert!(
            !open_against(ResponseTemplate::new(503))
                .await
                .err()
                .unwrap()
                .unreachable
        );
    }

    #[tokio::test]
    async fn a_slow_first_response_is_an_idle_timeout() {
        let result = open_against(sse(&["{}"]).set_delay(Duration::from_secs(2))).await;

        assert_eq!(result.err().map(|f| f.kind), Some(ErrorKind::Timeout));
    }

    #[test]
    fn reads_error_codes_from_either_shape() {
        let anthropic = serde_json::json!({"type": "error", "error": {"type": "overloaded_error"}});
        let openai = serde_json::json!({"code": "rate_limit_exceeded", "message": "x"});

        assert_eq!(
            stream_error(&anthropic).kind,
            ErrorKind::ProviderUnavailable
        );
        assert_eq!(stream_error(&openai).kind, ErrorKind::RateLimited);
        assert_eq!(
            stream_error(&serde_json::json!({})).kind,
            ErrorKind::ProviderUnavailable
        );
    }
}
