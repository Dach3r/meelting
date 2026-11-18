use serde::Deserialize;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Deserialize)]
struct Models {
    data: Vec<Model>,
}

#[derive(Deserialize)]
struct Model {
    id: String,
}

pub async fn list(base_url: &str) -> Result<Vec<String>, reqwest::Error> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let models: Models = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .build()?
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(models.data.into_iter().map(|model| model.id).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn lists_model_ids() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "object": "list",
                "data": [{ "id": "qwen2.5:0.5b" }, { "id": "llama3.2:3b" }],
            })))
            .mount(&server)
            .await;

        let models = list(&server.uri()).await.unwrap();

        assert_eq!(models, ["qwen2.5:0.5b", "llama3.2:3b"]);
    }

    #[tokio::test]
    async fn fails_on_error_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        assert!(list(&server.uri()).await.is_err());
    }

    #[tokio::test]
    async fn fails_on_malformed_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&server)
            .await;

        assert!(list(&server.uri()).await.is_err());
    }

    #[tokio::test]
    async fn fails_when_nothing_is_listening() {
        assert!(list("http://127.0.0.1:1").await.is_err());
    }
}
