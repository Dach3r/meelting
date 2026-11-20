use crate::domain::agent_run::ErrorKind;
use crate::domain::tools::{Definition, MAX_ARGUMENT_BYTES};
use crate::system::llm_stream::{
    self, Failure, OnText, StreamRequest, Usage, parse, stream_error, tokens,
};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

const STREAM_END: &str = "[DONE]";
const MAX_CALL_INDEX: usize = 64;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawCall {
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments: String,
    pub oversized: bool,
}

pub fn assistant_message(text: &str, calls: &[(&str, &str, &str)]) -> Value {
    let calls: Vec<Value> = calls
        .iter()
        .map(|(id, name, arguments)| {
            json!({
                "id": id,
                "type": "function",
                "function": { "name": name, "arguments": arguments }
            })
        })
        .collect();
    json!({
        "role": "assistant",
        "content": if text.is_empty() { Value::Null } else { json!(text) },
        "tool_calls": calls
    })
}

pub fn tool_message(call_id: &str, content: &str) -> Value {
    json!({ "role": "tool", "tool_call_id": call_id, "content": content })
}

fn tool_definition(definition: &Definition) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": definition.name,
            "description": definition.description,
            "parameters": definition.parameters
        }
    })
}

pub async fn stream(
    request: &StreamRequest<'_>,
    output_tokens_param: &str,
    on_text: &mut OnText<'_>,
) -> Result<Usage, Failure> {
    let (usage, calls) = stream_turn(request, output_tokens_param, &[], &[], on_text).await?;
    if calls.is_empty() {
        Ok(usage)
    } else {
        Err(ErrorKind::InvalidResponse.into())
    }
}

pub async fn stream_turn(
    request: &StreamRequest<'_>,
    output_tokens_param: &str,
    tools: &[Definition],
    exchange: &[Value],
    on_text: &mut OnText<'_>,
) -> Result<(Usage, Vec<RawCall>), Failure> {
    let mut messages = vec![json!({ "role": "system", "content": request.prompt.system })];
    messages.extend(
        request
            .prompt
            .turns
            .iter()
            .map(|turn| json!({ "role": turn.role.as_str(), "content": turn.text })),
    );
    messages.extend(exchange.iter().cloned());
    let mut body = Map::new();
    body.insert("model".into(), json!(request.model));
    body.insert("messages".into(), Value::Array(messages));
    body.insert(output_tokens_param.into(), json!(request.max_output_tokens));
    body.insert("stream".into(), json!(true));
    body.insert("stream_options".into(), json!({ "include_usage": true }));
    if !tools.is_empty() {
        body.insert(
            "tools".into(),
            Value::Array(tools.iter().map(tool_definition).collect()),
        );
    }
    let mut events =
        llm_stream::open(request, "/chat/completions", &[], Value::Object(body)).await?;

    let mut usage = Usage::default();
    let mut stopped = false;
    let mut calls: BTreeMap<usize, RawCall> = BTreeMap::new();
    while let Some(event) = events.next().await? {
        if event.data == STREAM_END {
            return if stopped {
                Ok((usage, calls.into_values().collect()))
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
            if let Some(fragments) = choice.pointer("/delta/tool_calls") {
                collect(&mut calls, fragments)?;
            }
            match choice.get("finish_reason").and_then(Value::as_str) {
                None => {}
                Some("stop") => stopped = true,
                Some("tool_calls") if !calls.is_empty() => stopped = true,
                Some("length") => return Err(ErrorKind::OutputTruncated.into()),
                Some("content_filter") => return Err(ErrorKind::ContentFiltered.into()),
                Some(_) => return Err(ErrorKind::InvalidResponse.into()),
            }
        }
    }
    if stopped {
        Ok((usage, calls.into_values().collect()))
    } else {
        Err(ErrorKind::ProviderUnavailable.into())
    }
}

fn collect(calls: &mut BTreeMap<usize, RawCall>, fragments: &Value) -> Result<(), Failure> {
    let invalid = || Failure::from(ErrorKind::InvalidResponse);
    for fragment in fragments.as_array().ok_or_else(invalid)? {
        let index = fragment
            .get("index")
            .and_then(Value::as_u64)
            .and_then(|index| usize::try_from(index).ok())
            .filter(|index| *index < MAX_CALL_INDEX)
            .ok_or_else(invalid)?;
        let call = calls.entry(index).or_default();
        if let Some(id) = fragment.get("id").and_then(Value::as_str)
            && !id.is_empty()
        {
            match &call.id {
                Some(known) if known != id => return Err(invalid()),
                _ => call.id = Some(id.to_string()),
            }
        }
        if let Some(name) = fragment.pointer("/function/name").and_then(Value::as_str)
            && !name.is_empty()
        {
            call.name.get_or_insert_with(|| name.to_string());
        }
        match fragment.pointer("/function/arguments") {
            Some(Value::String(text)) => append(call, text),
            Some(object @ Value::Object(_)) => append(call, &object.to_string()),
            _ => {}
        }
    }
    Ok(())
}

fn append(call: &mut RawCall, text: &str) {
    if call.oversized || call.arguments.len() + text.len() > MAX_ARGUMENT_BYTES {
        call.oversized = true;
        call.arguments.clear();
    } else {
        call.arguments.push_str(text);
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

    async fn turn_against(
        response: ResponseTemplate,
        exchange: &[Value],
    ) -> (String, Result<(Usage, Vec<RawCall>), Failure>) {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(response)
            .mount(&server)
            .await;
        let prompt = prompt();
        let uri = server.uri();
        let mut texts = Vec::new();
        let tools = crate::domain::tools::definitions();
        let result = stream_turn(
            &request(&uri, &prompt),
            "max_tokens",
            &tools,
            exchange,
            &mut collector(&mut texts),
        )
        .await;
        (texts.concat(), result)
    }

    fn tool_call_chunks(fragments: &[&str], finish: &str) -> Vec<String> {
        let mut events: Vec<String> = fragments
            .iter()
            .map(|fragment| format!(r#"{{"choices":[{{"delta":{{"tool_calls":[{fragment}]}}}}]}}"#))
            .collect();
        events.push(format!(
            r#"{{"choices":[{{"delta":{{}},"finish_reason":"{finish}"}}]}}"#
        ));
        events.push(r#"{"choices":[],"usage":{"prompt_tokens":9,"completion_tokens":4}}"#.into());
        events.push("[DONE]".into());
        events
    }

    fn sse_of(events: &[String]) -> ResponseTemplate {
        let refs: Vec<&str> = events.iter().map(String::as_str).collect();
        sse(&refs)
    }

    #[tokio::test]
    async fn assembles_fragmented_tool_calls_in_index_order() {
        let events = tool_call_chunks(
            &[
                r#"{"index":1,"id":"call_b","type":"function","function":{"name":"list_dir","arguments":""}}"#,
                r#"{"index":0,"id":"call_a","type":"function","function":{"name":"list_dir","arguments":"{\"pa"}}"#,
                r#"{"index":0,"function":{"arguments":"th\":\"src\"}"}}"#,
                r#"{"index":1,"function":{"arguments":"{}"}}"#,
            ],
            "tool_calls",
        );

        let (text, result) = turn_against(sse_of(&events), &[]).await;

        let (usage, calls) = result.unwrap();
        assert_eq!(text, "");
        assert_eq!(usage.input_tokens, Some(9));
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].id.as_deref(), Some("call_a"));
        assert_eq!(calls[0].name.as_deref(), Some("list_dir"));
        assert_eq!(calls[0].arguments, r#"{"path":"src"}"#);
        assert_eq!(calls[1].arguments, "{}");
        assert!(calls.iter().all(|call| !call.oversized));
    }

    #[tokio::test]
    async fn some_providers_finish_a_tool_turn_with_stop_and_send_arguments_as_an_object() {
        let events = tool_call_chunks(
            &[r#"{"index":0,"id":"c","function":{"name":"list_dir","arguments":{"path":"a"}}}"#],
            "stop",
        );

        let (_, result) = turn_against(sse_of(&events), &[]).await;

        assert_eq!(result.unwrap().1[0].arguments, r#"{"path":"a"}"#);
    }

    #[tokio::test]
    async fn sends_the_tool_definitions_and_the_exchange_after_the_prompt() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_partial_json(json!({
                "tools": [{ "type": "function", "function": { "name": "list_dir" } }],
                "messages": [
                    { "role": "system" }, { "role": "user", "content": "Hi" },
                    { "role": "assistant", "content": "Hello" }, { "role": "user", "content": "Status?" },
                    { "role": "assistant", "tool_calls": [{ "id": "c1", "type": "function",
                        "function": { "name": "list_dir", "arguments": "{}" } }] },
                    { "role": "tool", "tool_call_id": "c1", "content": "a\n" }
                ]
            })))
            .respond_with(sse(&DONE))
            .expect(1)
            .mount(&server)
            .await;
        let prompt = prompt();
        let uri = server.uri();
        let mut texts = Vec::new();
        let exchange = [
            assistant_message("", &[("c1", "list_dir", "{}")]),
            tool_message("c1", "a\n"),
        ];

        let (_, calls) = stream_turn(
            &request(&uri, &prompt),
            "max_tokens",
            &crate::domain::tools::definitions(),
            &exchange,
            &mut collector(&mut texts),
        )
        .await
        .unwrap();

        assert!(calls.is_empty());
        assert_eq!(texts.concat(), "Hello");
    }

    #[tokio::test]
    async fn the_text_only_entry_point_refuses_tool_calls() {
        let events = tool_call_chunks(
            &[r#"{"index":0,"id":"c","function":{"name":"list_dir","arguments":"{}"}}"#],
            "tool_calls",
        );

        let (_, result) = run_against(sse_of(&events), "max_tokens").await;

        assert_eq!(result.unwrap_err().kind, ErrorKind::InvalidResponse);
    }

    #[tokio::test]
    async fn refuses_malformed_tool_streams() {
        let cases: [(Vec<String>, ErrorKind); 5] = [
            (
                tool_call_chunks(&[], "tool_calls"),
                ErrorKind::InvalidResponse,
            ),
            (
                tool_call_chunks(
                    &[r#"{"index":64,"id":"c","function":{"name":"list_dir","arguments":"{}"}}"#],
                    "tool_calls",
                ),
                ErrorKind::InvalidResponse,
            ),
            (
                tool_call_chunks(
                    &[
                        r#"{"index":0,"id":"a","function":{"name":"list_dir","arguments":"{}"}}"#,
                        r#"{"index":0,"id":"b","function":{"arguments":""}}"#,
                    ],
                    "tool_calls",
                ),
                ErrorKind::InvalidResponse,
            ),
            (
                tool_call_chunks(&[r#"{"id":"a"}"#], "tool_calls"),
                ErrorKind::InvalidResponse,
            ),
            (
                tool_call_chunks(
                    &[r#"{"index":0,"id":"a","function":{"name":"list_dir","arguments":"{"}}"#],
                    "length",
                ),
                ErrorKind::OutputTruncated,
            ),
        ];

        for (events, expected) in cases {
            let (_, result) = turn_against(sse_of(&events), &[]).await;
            assert_eq!(result.unwrap_err().kind, expected);
        }
    }

    #[tokio::test]
    async fn a_tool_call_stream_cut_before_its_finish_is_an_unavailable_provider() {
        let events = [
            r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"a","function":{"name":"list_dir","arguments":"{"}}]}}]}"#,
        ];

        let (_, result) = turn_against(sse(&events), &[]).await;

        assert_eq!(result.unwrap_err().kind, ErrorKind::ProviderUnavailable);
    }

    #[tokio::test]
    async fn arguments_beyond_the_audit_limit_are_flagged_and_not_kept() {
        let long = "a".repeat(MAX_ARGUMENT_BYTES);
        let first = format!(
            r#"{{"index":0,"id":"a","function":{{"name":"list_dir","arguments":"{{\"path\":\"{long}"}}}}"#
        );
        let events = tool_call_chunks(
            &[&first, r#"{"index":0,"function":{"arguments":"x"}}"#],
            "tool_calls",
        );

        let (_, result) = turn_against(sse_of(&events), &[]).await;

        let call = &result.unwrap().1[0];
        assert!(call.oversized);
        assert!(call.arguments.is_empty());
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
    async fn streams_only_the_answer_of_a_deepseek_thinking_reply() {
        let (texts, result) = run_against(
            sse(&[
                r#"{"choices":[{"delta":{"role":"assistant","content":null,"reasoning_content":"Let me think"},"finish_reason":null}],"usage":null}"#,
                r#"{"choices":[{"delta":{"content":"Hi","reasoning_content":null},"finish_reason":null}],"usage":null}"#,
                r#"{"choices":[{"delta":{"content":""},"finish_reason":"stop"}],"usage":{"prompt_tokens":9,"completion_tokens":4}}"#,
                "[DONE]",
            ]),
            "max_tokens",
        )
        .await;

        assert_eq!(texts, ["Hi"]);
        assert_eq!(
            result,
            Ok(Usage {
                input_tokens: Some(9),
                output_tokens: Some(4)
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
