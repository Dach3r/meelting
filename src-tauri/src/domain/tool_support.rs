use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSupport {
    Supported,
    Unsupported,
    Unknown,
}

pub fn from_ollama_show(body: &Value) -> ToolSupport {
    let Some(capabilities) = body.get("capabilities").and_then(Value::as_array) else {
        return ToolSupport::Unknown;
    };
    let names: Option<Vec<&str>> = capabilities.iter().map(Value::as_str).collect();
    match names {
        Some(names) if names.contains(&"tools") => ToolSupport::Supported,
        Some(names) if !names.is_empty() => ToolSupport::Unsupported,
        _ => ToolSupport::Unknown,
    }
}

pub fn from_lmstudio_models(body: &Value, model: &str) -> ToolSupport {
    let Some(models) = body.get("models").and_then(Value::as_array) else {
        return ToolSupport::Unknown;
    };
    let named =
        |entry: &&Value, field: &str| entry.get(field).and_then(Value::as_str) == Some(model);
    let Some(entry) = models
        .iter()
        .find(|entry| named(entry, "key") || named(entry, "selected_variant"))
    else {
        return ToolSupport::Unknown;
    };
    match entry
        .pointer("/capabilities/trained_for_tool_use")
        .and_then(Value::as_bool)
    {
        Some(true) => ToolSupport::Supported,
        Some(false) => ToolSupport::Unsupported,
        None => ToolSupport::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ollama_reports_tools_only_when_its_capabilities_list_says_so() {
        let cases = [
            (
                json!({"capabilities": ["completion", "tools"]}),
                ToolSupport::Supported,
            ),
            (json!({"capabilities": ["tools"]}), ToolSupport::Supported),
            (
                json!({"capabilities": ["completion", "vision"]}),
                ToolSupport::Unsupported,
            ),
            (
                json!({"capabilities": ["completion", "tool"]}),
                ToolSupport::Unsupported,
            ),
            (json!({"capabilities": []}), ToolSupport::Unknown),
            (
                json!({"capabilities": ["completion", 3]}),
                ToolSupport::Unknown,
            ),
            (json!({"capabilities": "tools"}), ToolSupport::Unknown),
            (json!({"capabilities": null}), ToolSupport::Unknown),
            (
                json!({"details": {"family": "llama"}, "template": "{{ .Tools }}"}),
                ToolSupport::Unknown,
            ),
            (json!([]), ToolSupport::Unknown),
        ];
        for (body, expected) in cases {
            assert_eq!(from_ollama_show(&body), expected, "{body}");
        }
    }

    #[test]
    fn lm_studio_uses_only_the_native_trained_for_tool_use_flag_of_the_named_model() {
        let body = json!({"models": [
            {"type": "llm", "key": "qwen/qwen3-4b", "capabilities": {"vision": false, "trained_for_tool_use": true}},
            {"type": "llm", "key": "google/gemma", "capabilities": {"vision": false, "trained_for_tool_use": false}},
            {"type": "llm", "key": "other/model", "capabilities": {"vision": false}},
            {"type": "llm", "key": "bare/model"},
            {"type": "llm", "key": "str/model", "capabilities": {"trained_for_tool_use": "true"}},
            {"type": "llm", "key": "variant", "selected_variant": "variant@q4", "capabilities": {"trained_for_tool_use": true}},
        ]});
        let cases = [
            ("qwen/qwen3-4b", ToolSupport::Supported),
            ("google/gemma", ToolSupport::Unsupported),
            ("other/model", ToolSupport::Unknown),
            ("bare/model", ToolSupport::Unknown),
            ("str/model", ToolSupport::Unknown),
            ("variant@q4", ToolSupport::Supported),
            ("missing/model", ToolSupport::Unknown),
            ("Qwen/Qwen3-4B", ToolSupport::Unknown),
        ];
        for (model, expected) in cases {
            assert_eq!(from_lmstudio_models(&body, model), expected, "{model}");
        }
        assert_eq!(
            from_lmstudio_models(&json!({"data": []}), "x"),
            ToolSupport::Unknown
        );
        assert_eq!(
            from_lmstudio_models(&json!({"models": {}}), "x"),
            ToolSupport::Unknown
        );
    }

    #[test]
    fn the_model_name_or_family_never_decides_anything() {
        let body = json!({"models": [{"key": "llama-3.1-8b-instruct-tools-tool-use"}]});

        assert_eq!(
            from_lmstudio_models(&body, "llama-3.1-8b-instruct-tools-tool-use"),
            ToolSupport::Unknown
        );
        assert_eq!(
            from_ollama_show(&json!({"details": {"family": "qwen2.5-tools"}})),
            ToolSupport::Unknown
        );
    }
}
