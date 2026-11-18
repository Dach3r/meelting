use crate::domain::agent_run::ErrorKind;
use crate::system::llm_stream::{
    self, Failure, OnText, StreamRequest, Usage, parse, stream_error, tokens,
};
use serde_json::{Value, json};

const API_VERSION: &str = "2026-10-21";

pub async fn stream(
    request: &StreamRequest<'_>,
    on_text: &mut OnText<'_>,
) -> Result<Usage, Failure> {
    let messages: Vec<Value> = request
        .prompt
        .turns
        .iter()
        .map(|turn| json!({ "role": turn.role.as_str(), "content": turn.text }))
        .collect();
    let body = json!({
        "model": request.model,
        "max_tokens": request.max_output_tokens,
        "system": request.prompt.system,
        "messages": messages,
        "stream": true,
    });
    let mut events = llm_stream::open(
        request,
        "/messages",
        &[("anthropic-version", API_VERSION)],
        body,
    )
    .await?;

    let mut usage = Usage::default();
    let mut stopped = false;
    while let Some(event) = events.next().await? {
        let data = parse(&event.data)?;
        match data.get("type").and_then(Value::as_str) {
            Some("message_start") => {
                if let Some(reported) = data.pointer("/message/usage") {
                    usage.input_tokens = tokens(reported, "input_tokens");
                    usage.output_tokens = tokens(reported, "output_tokens");
                }
            }
            Some("content_block_delta") => {
                if data.pointer("/delta/type").and_then(Value::as_str) == Some("text_delta")
                    && let Some(text) = data.pointer("/delta/text").and_then(Value::as_str)
                    && !text.is_empty()
                {
                    on_text(text)?;
                }
            }
            Some("message_delta") => {
                if let Some(reported) = data.get("usage") {
                    usage.output_tokens = tokens(reported, "output_tokens").or(usage.output_tokens);
                    usage.input_tokens = tokens(reported, "input_tokens").or(usage.input_tokens);
                }
                match data.pointer("/delta/stop_reason").and_then(Value::as_str) {
                    None => {}
                    Some("end_turn" | "stop_sequence") => stopped = true,
                    Some("max_tokens" | "model_context_window_exceeded") => {
                        return Err(ErrorKind::OutputTruncated.into());
                    }
                    Some("refusal") => return Err(ErrorKind::ContentFiltered.into()),
                    Some(_) => return Err(ErrorKind::InvalidResponse.into()),
                }
            }
            Some("message_stop") => {
                return if stopped {
                    Ok(usage)
                } else {
                    Err(ErrorKind::InvalidResponse.into())
                };
            }
            Some("error") => return Err(stream_error(&data)),
            _ => {}
        }
    }
    Err(ErrorKind::ProviderUnavailable.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::llm_stream::testing::{collector, prompt, request, sse};
    use std::time::Duration;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const START: &str =
        r#"{"type":"message_start","message":{"usage":{"input_tokens":21,"output_tokens":1}}}"#;
    const PING: &str = r#"{"type":"ping"}"#;
    const HEL: &str =
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hel"}}"#;
    const LO: &str =
        r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"lo"}}"#;
    const END: &str = r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":7}}"#;
    const STOP: &str = r#"{"type":"message_stop"}"#;

    async fn run_against(response: ResponseTemplate) -> (Vec<String>, Result<Usage, Failure>) {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(response)
            .mount(&server)
            .await;
        let prompt = prompt();
        let uri = server.uri();
        let mut texts = Vec::new();
        let result = stream(&request(&uri, &prompt), &mut collector(&mut texts)).await;
        (texts, result)
    }

    #[tokio::test]
    async fn sends_system_turns_and_the_fixed_api_version() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/messages"))
            .and(header("authorization", "Bearer sk-test"))
            .and(header("anthropic-version", API_VERSION))
            .and(body_json(json!({
                "model": "test-model",
                "max_tokens": 256,
                "system": "Be brief.",
                "messages": [
                    { "role": "user", "content": "Hi" },
                    { "role": "assistant", "content": "Hello" },
                    { "role": "user", "content": "Status?" },
                ],
                "stream": true,
            })))
            .respond_with(sse(&[START, HEL, END, STOP]))
            .expect(1)
            .mount(&server)
            .await;
        let prompt = prompt();
        let uri = server.uri();

        let result = stream(&request(&uri, &prompt), &mut |_| Ok(())).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn streams_text_ignores_pings_and_reads_cumulative_usage() {
        let (texts, result) = run_against(sse(&[
            START,
            PING,
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
            HEL,
            r#"{"type":"unknown_future_event"}"#,
            LO,
            END,
            STOP,
        ]))
        .await;

        assert_eq!(texts, ["Hel", "lo"]);
        assert_eq!(
            result,
            Ok(Usage {
                input_tokens: Some(21),
                output_tokens: Some(7)
            })
        );
    }

    #[tokio::test]
    async fn maps_each_stop_reason() {
        for (reason, kind) in [
            ("max_tokens", ErrorKind::OutputTruncated),
            ("model_context_window_exceeded", ErrorKind::OutputTruncated),
            ("refusal", ErrorKind::ContentFiltered),
            ("tool_use", ErrorKind::InvalidResponse),
            ("pause_turn", ErrorKind::InvalidResponse),
        ] {
            let delta =
                format!(r#"{{"type":"message_delta","delta":{{"stop_reason":"{reason}"}}}}"#);
            let (_, result) = run_against(sse(&[START, HEL, &delta, STOP])).await;
            assert_eq!(result.map_err(|f| f.kind), Err(kind), "{reason}");
        }
    }

    #[tokio::test]
    async fn an_overloaded_error_in_the_stream_is_transient() {
        let (_, result) = run_against(sse(&[
            START,
            r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#,
        ]))
        .await;

        assert_eq!(
            result.map_err(|f| f.kind),
            Err(ErrorKind::ProviderUnavailable)
        );
    }

    #[tokio::test]
    async fn a_stream_cut_before_message_stop_is_unavailable() {
        let (texts, result) = run_against(sse(&[START, HEL])).await;

        assert_eq!(texts, ["Hel"]);
        assert_eq!(
            result.map_err(|f| f.kind),
            Err(ErrorKind::ProviderUnavailable)
        );
    }

    #[tokio::test]
    async fn maps_http_errors_including_529() {
        for (status, kind) in [
            (401, ErrorKind::ProviderAuthFailed),
            (404, ErrorKind::ModelUnavailable),
            (429, ErrorKind::RateLimited),
            (529, ErrorKind::ProviderUnavailable),
            (400, ErrorKind::ProviderRejected),
        ] {
            let (_, result) = run_against(ResponseTemplate::new(status)).await;
            assert_eq!(result.map_err(|f| f.kind), Err(kind), "{status}");
        }
    }

    #[tokio::test]
    async fn rejects_malformed_events() {
        let (_, result) = run_against(sse(&["{nope"])).await;

        assert_eq!(result.map_err(|f| f.kind), Err(ErrorKind::InvalidResponse));
    }

    #[tokio::test]
    #[ignore = "needs MEELTING_ANTHROPIC_API_KEY; calls the real Anthropic API"]
    async fn streams_from_the_real_api() {
        let key = std::env::var("MEELTING_ANTHROPIC_API_KEY").unwrap();
        let prompt = prompt();
        let mut request = request("https://api.anthropic.com/v1", &prompt);
        request.api_key = Some(&key);
        request.model = "claude-haiku-5-5";
        request.idle_timeout = Duration::from_secs(45);
        let mut texts = Vec::new();

        let usage = stream(&request, &mut collector(&mut texts)).await.unwrap();

        assert!(!texts.concat().is_empty());
        assert!(usage.output_tokens.is_some());
    }
}
