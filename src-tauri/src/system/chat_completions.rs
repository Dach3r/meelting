use crate::domain::agent_run::ErrorKind;
use crate::system::llm_stream::{
    self, Failure, OnText, StreamRequest, Usage, parse, stream_error, tokens,
};
use serde_json::{Map, Value, json};

const STREAM_END: &str = "[DONE]";

pub async fn stream(
    request: &StreamRequest<'_>,
    output_tokens_param: &str,
    on_text: &mut OnText<'_>,
) -> Result<Usage, Failure> {
    let mut messages = vec![json!({ "role": "system", "content": request.prompt.system })];
    messages.extend(
        request
            .prompt
            .turns
            .iter()
            .map(|turn| json!({ "role": turn.role.as_str(), "content": turn.text })),
    );
    let mut body = Map::new();
    body.insert("model".into(), json!(request.model));
    body.insert("messages".into(), Value::Array(messages));
    body.insert(output_tokens_param.into(), json!(request.max_output_tokens));
    body.insert("stream".into(), json!(true));
    body.insert("stream_options".into(), json!({ "include_usage": true }));
    let mut events =
        llm_stream::open(request, "/chat/completions", &[], Value::Object(body)).await?;

    let mut usage = Usage::default();
    let mut stopped = false;
    while let Some(event) = events.next().await? {
        if event.data == STREAM_END {
            return if stopped {
                Ok(usage)
            } else {
                Err(ErrorKind::InvalidResponse.into())
            };
        }
        let chunk = parse(&event.data)?;
        if chunk.get("error").is_some() {
            return Err(stream_error(&chunk));
        }
        let reported = chunk
            .get("usage")
            .filter(|value| value.is_object())
            .or_else(|| chunk.pointer("/x_groq/usage"));
        if let Some(reported) = reported {
            usage = Usage {
                input_tokens: tokens(reported, "prompt_tokens"),
                output_tokens: tokens(reported, "completion_tokens"),
            };
        }
        for choice in chunk
            .get("choices")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(text) = choice.pointer("/delta/content").and_then(Value::as_str)
                && !text.is_empty()
            {
                on_text(text)?;
            }
            match choice.get("finish_reason").and_then(Value::as_str) {
                None => {}
                Some("stop") => stopped = true,
                Some("length") => return Err(ErrorKind::OutputTruncated.into()),
                Some("content_filter") => return Err(ErrorKind::ContentFiltered.into()),
                Some(_) => return Err(ErrorKind::InvalidResponse.into()),
            }
        }
    }
    if stopped {
        Ok(usage)
    } else {
        Err(ErrorKind::ProviderUnavailable.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::llm_stream::testing::{collector, prompt, request, sse};
    use std::time::Duration;
    use wiremock::matchers::{body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn run_against(
        response: ResponseTemplate,
        param: &str,
    ) -> (Vec<String>, Result<Usage, Failure>) {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(response)
            .mount(&server)
            .await;
        let prompt = prompt();
        let uri = server.uri();
        let mut texts = Vec::new();
        let result = stream(&request(&uri, &prompt), param, &mut collector(&mut texts)).await;
        (texts, result)
    }

    const DONE: [&str; 4] = [
        r#"{"choices":[{"delta":{"role":"assistant","content":""}}]}"#,
        r#"{"choices":[{"delta":{"content":"Hel"}}]}"#,
        r#"{"choices":[{"delta":{"content":"lo"},"finish_reason":"stop"}]}"#,
        "[DONE]",
    ];

    #[tokio::test]
    async fn sends_the_prompt_with_the_catalog_output_parameter() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer sk-test"))
            .and(body_partial_json(json!({
                "model": "test-model",
                "stream": true,
                "stream_options": { "include_usage": true },
                "max_completion_tokens": 256,
                "messages": [
                    { "role": "system", "content": "Be brief." },
                    { "role": "user", "content": "Hi" },
                    { "role": "assistant", "content": "Hello" },
                    { "role": "user", "content": "Status?" },
                ],
            })))
            .respond_with(sse(&DONE))
            .expect(1)
            .mount(&server)
            .await;
        let prompt = prompt();
        let uri = server.uri();
        let mut texts = Vec::new();

        let result = stream(
            &request(&uri, &prompt),
            "max_completion_tokens",
            &mut collector(&mut texts),
        )
        .await;

        assert_eq!(result, Ok(Usage::default()));
        assert_eq!(texts.concat(), "Hello");
    }

    #[tokio::test]
    async fn streams_text_in_order_and_reads_the_final_usage_chunk() {
        let (texts, result) = run_against(
            sse(&[
                DONE[1],
                DONE[2],
                r#"{"choices":[],"usage":{"prompt_tokens":12,"completion_tokens":3}}"#,
                "[DONE]",
            ]),
            "max_tokens",
        )
        .await;

        assert_eq!(texts, ["Hel", "lo"]);
        assert_eq!(
            result,
            Ok(Usage {
                input_tokens: Some(12),
                output_tokens: Some(3)
            })
        );
    }

    #[tokio::test]
    async fn reads_groq_usage_from_x_groq() {
        let (_, result) = run_against(
            sse(&[
                r#"{"choices":[{"delta":{"content":"ok"},"finish_reason":"stop"}],"x_groq":{"usage":{"prompt_tokens":5,"completion_tokens":1}}}"#,
                "[DONE]",
            ]),
            "max_completion_tokens",
        )
        .await;

        assert_eq!(
            result,
            Ok(Usage {
                input_tokens: Some(5),
                output_tokens: Some(1)
            })
        );
    }

    #[tokio::test]
    async fn maps_each_finish_reason() {
        for (reason, kind) in [
            ("length", ErrorKind::OutputTruncated),
            ("content_filter", ErrorKind::ContentFiltered),
            ("tool_calls", ErrorKind::InvalidResponse),
        ] {
            let chunk = format!(r#"{{"choices":[{{"delta":{{}},"finish_reason":"{reason}"}}]}}"#);
            let (_, result) = run_against(sse(&[&chunk, "[DONE]"]), "max_tokens").await;
            assert_eq!(result.map_err(|f| f.kind), Err(kind), "{reason}");
        }
    }

    #[tokio::test]
    async fn done_without_a_stop_is_invalid_and_a_cut_stream_is_unavailable() {
        let (_, done_early) = run_against(sse(&[DONE[1], "[DONE]"]), "max_tokens").await;
        let (texts, cut) = run_against(sse(&[DONE[1]]), "max_tokens").await;

        assert_eq!(
            done_early.map_err(|f| f.kind),
            Err(ErrorKind::InvalidResponse)
        );
        assert_eq!(texts, ["Hel"]);
        assert_eq!(cut.map_err(|f| f.kind), Err(ErrorKind::ProviderUnavailable));
    }

    #[tokio::test]
    async fn rejects_malformed_json_and_reports_stream_errors() {
        let (_, malformed) = run_against(sse(&["not json"]), "max_tokens").await;
        let (_, errored) = run_against(
            sse(&[r#"{"error":{"message":"secret detail","type":"server_error"}}"#]),
            "max_tokens",
        )
        .await;

        assert_eq!(
            malformed.map_err(|f| f.kind),
            Err(ErrorKind::InvalidResponse)
        );
        assert_eq!(
            errored.map_err(|f| f.kind),
            Err(ErrorKind::ProviderUnavailable)
        );
    }

    #[tokio::test]
    async fn maps_http_errors_without_provider_text() {
        for (status, kind) in [
            (401, ErrorKind::ProviderAuthFailed),
            (404, ErrorKind::ModelUnavailable),
            (429, ErrorKind::RateLimited),
            (500, ErrorKind::ProviderUnavailable),
            (400, ErrorKind::ProviderRejected),
        ] {
            let (texts, result) = run_against(
                ResponseTemplate::new(status).set_body_string("secret detail"),
                "max_tokens",
            )
            .await;
            assert!(texts.is_empty());
            assert_eq!(result.map_err(|f| f.kind), Err(kind), "{status}");
        }
    }

    #[tokio::test]
    async fn a_stream_that_goes_silent_is_an_idle_timeout() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(sse(&DONE).set_delay(Duration::from_secs(2)))
            .mount(&server)
            .await;
        let prompt = prompt();
        let uri = server.uri();

        let result = stream(&request(&uri, &prompt), "max_tokens", &mut |_| Ok(())).await;

        assert_eq!(result.map_err(|f| f.kind), Err(ErrorKind::Timeout));
    }

    #[tokio::test]
    async fn stops_when_the_text_callback_refuses_more_text() {
        let (_, result) = {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .respond_with(sse(&DONE))
                .mount(&server)
                .await;
            let prompt = prompt();
            let uri = server.uri();
            let result = stream(&request(&uri, &prompt), "max_tokens", &mut |_| {
                Err(ErrorKind::OutputTruncated.into())
            })
            .await;
            ((), result)
        };

        assert_eq!(result.map_err(|f| f.kind), Err(ErrorKind::OutputTruncated));
    }

    #[tokio::test]
    #[ignore = "needs MEELTING_LOCAL_BASE_URL and MEELTING_LOCAL_MODEL pointing at a running local provider"]
    async fn streams_from_real_local_provider() {
        let base_url = std::env::var("MEELTING_LOCAL_BASE_URL").unwrap();
        let model = std::env::var("MEELTING_LOCAL_MODEL").unwrap();
        let prompt = prompt();
        let mut request = request(&base_url, &prompt);
        request.model = &model;
        request.api_key = None;
        request.idle_timeout = Duration::from_secs(120);
        let mut texts = Vec::new();

        let usage = stream(&request, "max_tokens", &mut collector(&mut texts))
            .await
            .unwrap();

        assert!(!texts.concat().is_empty());
        println!("usage: {usage:?}");
    }
}
