use crate::domain::agent_run::ErrorKind;
use crate::system::llm_stream::{
    self, Failure, OnText, StreamRequest, Usage, parse, stream_error, tokens,
};
use serde_json::{Value, json};

pub async fn stream(
    request: &StreamRequest<'_>,
    on_text: &mut OnText<'_>,
) -> Result<Usage, Failure> {
    let input: Vec<Value> = request
        .prompt
        .turns
        .iter()
        .map(|turn| json!({ "role": turn.role.as_str(), "content": turn.text }))
        .collect();
    let body = json!({
        "model": request.model,
        "instructions": request.prompt.system,
        "input": input,
        "max_output_tokens": request.max_output_tokens,
        "store": false,
        "stream": true,
    });
    let mut events = llm_stream::open(request, "/responses", &[], body).await?;

    while let Some(event) = events.next().await? {
        let data = parse(&event.data)?;
        match data.get("type").and_then(Value::as_str) {
            Some("response.output_text.delta") => {
                if let Some(text) = data.get("delta").and_then(Value::as_str)
                    && !text.is_empty()
                {
                    on_text(text)?;
                }
            }
            Some("response.completed") => {
                let usage = data.pointer("/response/usage");
                return Ok(Usage {
                    input_tokens: usage.and_then(|usage| tokens(usage, "input_tokens")),
                    output_tokens: usage.and_then(|usage| tokens(usage, "output_tokens")),
                });
            }
            Some("response.incomplete") => {
                let reason = data
                    .pointer("/response/incomplete_details/reason")
                    .and_then(Value::as_str);
                return Err(match reason {
                    Some("max_output_tokens") => ErrorKind::OutputTruncated,
                    Some("content_filter") => ErrorKind::ContentFiltered,
                    _ => ErrorKind::InvalidResponse,
                }
                .into());
            }
            Some("response.failed") => {
                return Err(data
                    .pointer("/response/error")
                    .map_or(ErrorKind::ProviderUnavailable.into(), stream_error));
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

    const CREATED: &str = r#"{"type":"response.created","response":{"status":"in_progress"}}"#;
    const HEL: &str = r#"{"type":"response.output_text.delta","delta":"Hel"}"#;
    const LO: &str = r#"{"type":"response.output_text.delta","delta":"lo"}"#;
    const COMPLETED: &str = r#"{"type":"response.completed","response":{"status":"completed","usage":{"input_tokens":30,"output_tokens":9}}}"#;

    async fn run_against(response: ResponseTemplate) -> (Vec<String>, Result<Usage, Failure>) {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/responses"))
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
    async fn sends_instructions_input_and_no_storage() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(header("authorization", "Bearer sk-test"))
            .and(body_json(json!({
                "model": "test-model",
                "instructions": "Be brief.",
                "input": [
                    { "role": "user", "content": "Hi" },
                    { "role": "assistant", "content": "Hello" },
                    { "role": "user", "content": "Status?" },
                ],
                "max_output_tokens": 256,
                "store": false,
                "stream": true,
            })))
            .respond_with(sse(&[CREATED, HEL, COMPLETED]))
            .expect(1)
            .mount(&server)
            .await;
        let prompt = prompt();
        let uri = server.uri();

        let result = stream(&request(&uri, &prompt), &mut |_| Ok(())).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn streams_text_and_reads_usage_from_the_completed_event() {
        let (texts, result) = run_against(sse(&[CREATED, HEL, LO, COMPLETED])).await;

        assert_eq!(texts, ["Hel", "lo"]);
        assert_eq!(
            result,
            Ok(Usage {
                input_tokens: Some(30),
                output_tokens: Some(9)
            })
        );
    }

    #[tokio::test]
    async fn maps_each_incomplete_reason() {
        for (reason, kind) in [
            ("max_output_tokens", ErrorKind::OutputTruncated),
            ("content_filter", ErrorKind::ContentFiltered),
            ("other", ErrorKind::InvalidResponse),
        ] {
            let event = format!(
                r#"{{"type":"response.incomplete","response":{{"incomplete_details":{{"reason":"{reason}"}}}}}}"#
            );
            let (_, result) = run_against(sse(&[HEL, &event])).await;
            assert_eq!(result.map_err(|f| f.kind), Err(kind), "{reason}");
        }
    }

    #[tokio::test]
    async fn classifies_failures_by_code() {
        let (_, failed) = run_against(sse(&[
            r#"{"type":"response.failed","response":{"error":{"code":"server_error","message":"secret"}}}"#,
        ]))
        .await;
        let (_, errored) = run_against(sse(&[
            r#"{"type":"error","code":"rate_limit_exceeded","message":"secret"}"#,
        ]))
        .await;

        assert_eq!(
            failed.map_err(|f| f.kind),
            Err(ErrorKind::ProviderUnavailable)
        );
        assert_eq!(errored.map_err(|f| f.kind), Err(ErrorKind::RateLimited));
    }

    #[tokio::test]
    async fn an_exhausted_quota_is_billing() {
        let (_, result) = run_against(
            ResponseTemplate::new(429)
                .set_body_string(r#"{"error":{"code":"insufficient_quota","message":"secret"}}"#),
        )
        .await;

        assert_eq!(result.map_err(|f| f.kind), Err(ErrorKind::ProviderBilling));
    }

    #[tokio::test]
    async fn a_stream_cut_before_completion_is_unavailable() {
        let (texts, result) = run_against(sse(&[CREATED, HEL])).await;

        assert_eq!(texts, ["Hel"]);
        assert_eq!(
            result.map_err(|f| f.kind),
            Err(ErrorKind::ProviderUnavailable)
        );
    }

    #[tokio::test]
    #[ignore = "needs MEELTING_OPENAI_API_KEY; calls the real OpenAI API"]
    async fn streams_from_the_real_api() {
        let key = std::env::var("MEELTING_OPENAI_API_KEY").unwrap();
        let prompt = prompt();
        let mut request = request("https://api.openai.com/v1", &prompt);
        request.api_key = Some(&key);
        request.model = "gpt-6.1-terra";
        request.idle_timeout = Duration::from_secs(45);
        let mut texts = Vec::new();

        stream(&request, &mut collector(&mut texts)).await.unwrap();

        assert!(!texts.concat().is_empty());
    }
}
