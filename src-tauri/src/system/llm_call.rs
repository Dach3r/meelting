use crate::catalogs::llm::{Protocol, Provider};
use crate::system::llm_stream::{Failure, OnText, StreamRequest, Usage};
use crate::system::{anthropic_messages, chat_completions, openai_responses};

pub async fn stream(
    provider: &Provider,
    request: &StreamRequest<'_>,
    on_text: &mut OnText<'_>,
) -> Result<Usage, Failure> {
    match provider.protocol {
        Protocol::AnthropicMessages => anthropic_messages::stream(request, on_text).await,
        Protocol::OpenaiResponses => openai_responses::stream(request, on_text).await,
        Protocol::ChatCompletions => {
            let output_tokens_param = provider
                .output_tokens_param
                .map_or("max_tokens", |param| param.as_str());
            chat_completions::stream(request, output_tokens_param, on_text).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogs::llm::{Location, OutputTokensParam};
    use crate::system::llm_stream::testing::{prompt, request};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn provider(protocol: Protocol, base_url: String) -> Provider {
        Provider {
            key: "test".into(),
            name: "Test".into(),
            location: Location::Cloud,
            protocol,
            base_url,
            output_tokens_param: (protocol == Protocol::ChatCompletions)
                .then_some(OutputTokensParam::MaxCompletionTokens),
            models: vec![],
        }
    }

    #[tokio::test]
    async fn calls_the_endpoint_of_each_protocol() {
        for (protocol, endpoint) in [
            (Protocol::AnthropicMessages, "/messages"),
            (Protocol::OpenaiResponses, "/responses"),
            (Protocol::ChatCompletions, "/chat/completions"),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path(endpoint))
                .respond_with(ResponseTemplate::new(401))
                .expect(1)
                .mount(&server)
                .await;
            let prompt = prompt();
            let uri = server.uri();

            let result = stream(
                &provider(protocol, uri.clone()),
                &request(&uri, &prompt),
                &mut |_| Ok(()),
            )
            .await;

            assert_eq!(
                result.err().map(|failure| failure.kind),
                Some(crate::domain::agent_run::ErrorKind::ProviderAuthFailed),
                "{endpoint}"
            );
        }
    }
}
