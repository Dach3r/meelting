use crate::catalogs::llm::Location;
use serde::Serialize;
use std::time::Duration;
use uuid::Uuid;

pub const MAX_ATTEMPTS: i64 = 3;
pub const TOKEN_FLUSH: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Queued,
    Running,
    Succeeded,
    Passed,
    Failed,
}

impl RunStatus {
    pub fn parse(value: &str) -> Option<RunStatus> {
        match value {
            "queued" => Some(RunStatus::Queued),
            "running" => Some(RunStatus::Running),
            "succeeded" => Some(RunStatus::Succeeded),
            "passed" => Some(RunStatus::Passed),
            "failed" => Some(RunStatus::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    RateLimited,
    Timeout,
    ProviderUnavailable,
    ProviderNotConfigured,
    ModelUnavailable,
    ProviderAuthFailed,
    ProviderBilling,
    ProviderRejected,
    ContentFiltered,
    OutputTruncated,
    InvalidResponse,
    ContextTooLarge,
    Cancelled,
    Expired,
    ToolsUnsupported,
    IterationBudgetExhausted,
    ToolCallBudgetExhausted,
    ToolOutputBudgetExhausted,
}

const ERROR_KINDS: [ErrorKind; 18] = [
    ErrorKind::RateLimited,
    ErrorKind::Timeout,
    ErrorKind::ProviderUnavailable,
    ErrorKind::ProviderNotConfigured,
    ErrorKind::ModelUnavailable,
    ErrorKind::ProviderAuthFailed,
    ErrorKind::ProviderBilling,
    ErrorKind::ProviderRejected,
    ErrorKind::ContentFiltered,
    ErrorKind::OutputTruncated,
    ErrorKind::InvalidResponse,
    ErrorKind::ContextTooLarge,
    ErrorKind::Cancelled,
    ErrorKind::Expired,
    ErrorKind::ToolsUnsupported,
    ErrorKind::IterationBudgetExhausted,
    ErrorKind::ToolCallBudgetExhausted,
    ErrorKind::ToolOutputBudgetExhausted,
];

impl ErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorKind::RateLimited => "rate_limited",
            ErrorKind::Timeout => "timeout",
            ErrorKind::ProviderUnavailable => "provider_unavailable",
            ErrorKind::ProviderNotConfigured => "provider_not_configured",
            ErrorKind::ModelUnavailable => "model_unavailable",
            ErrorKind::ProviderAuthFailed => "provider_auth_failed",
            ErrorKind::ProviderBilling => "provider_billing",
            ErrorKind::ProviderRejected => "provider_rejected",
            ErrorKind::ContentFiltered => "content_filtered",
            ErrorKind::OutputTruncated => "output_truncated",
            ErrorKind::InvalidResponse => "invalid_response",
            ErrorKind::ContextTooLarge => "context_too_large",
            ErrorKind::Cancelled => "cancelled",
            ErrorKind::Expired => "expired",
            ErrorKind::ToolsUnsupported => "tools_unsupported",
            ErrorKind::IterationBudgetExhausted => "iteration_budget_exhausted",
            ErrorKind::ToolCallBudgetExhausted => "tool_call_budget_exhausted",
            ErrorKind::ToolOutputBudgetExhausted => "tool_output_budget_exhausted",
        }
    }

    pub fn parse(value: &str) -> Option<ErrorKind> {
        ERROR_KINDS.into_iter().find(|kind| kind.as_str() == value)
    }

    pub fn is_transient(self) -> bool {
        matches!(
            self,
            ErrorKind::RateLimited | ErrorKind::Timeout | ErrorKind::ProviderUnavailable
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentRun {
    pub id: String,
    pub room_id: String,
    pub agent_id: String,
    pub request_message_id: String,
    pub response_message_id: Option<String>,
    pub status: RunStatus,
    pub error_kind: Option<ErrorKind>,
    pub attempts: i64,
    pub provider: String,
    pub model: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Partial {
    pub run_id: String,
    pub attempt: i64,
    pub seq: u64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TokenChunk {
    pub run_id: String,
    pub room_id: String,
    pub attempt: i64,
    pub seq: u64,
    pub text: String,
}

pub fn is_pass(reply: &str) -> bool {
    reply
        .trim_matches(|character: char| !character.is_alphanumeric())
        .eq_ignore_ascii_case("pass")
}

pub fn is_canonical_client_request_id(value: &str) -> bool {
    Uuid::parse_str(value).is_ok_and(|uuid| uuid.hyphenated().to_string() == value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocationLimits {
    pub output_tokens: u32,
    pub prompt_chars: usize,
    pub history_messages: usize,
    pub connect_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_duration: Duration,
    pub concurrency: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub cloud: LocationLimits,
    pub local: LocationLimits,
    pub backoff: [Duration; 2],
    pub max_retry_after: Duration,
    pub max_response_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            cloud: LocationLimits {
                output_tokens: 16_000,
                prompt_chars: 24_000,
                history_messages: 100,
                connect_timeout: Duration::from_secs(10),
                idle_timeout: Duration::from_secs(45),
                max_duration: Duration::from_secs(15 * 60),
                concurrency: 4,
            },
            local: LocationLimits {
                output_tokens: 2048,
                prompt_chars: 8000,
                history_messages: 100,
                connect_timeout: Duration::from_secs(10),
                idle_timeout: Duration::from_secs(120),
                max_duration: Duration::from_secs(45 * 60),
                concurrency: 1,
            },
            backoff: [Duration::from_secs(2), Duration::from_secs(8)],
            max_retry_after: Duration::from_secs(60),
            max_response_bytes: 256 * 1024,
        }
    }
}

impl Limits {
    pub fn at(&self, location: Location) -> &LocationLimits {
        match location {
            Location::Cloud => &self.cloud,
            Location::Local => &self.local,
        }
    }

    pub fn output_tokens(&self, location: Location, model_max: Option<u32>) -> u32 {
        let cap = self.at(location).output_tokens;
        model_max.map_or(cap, |max| max.min(cap))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextStep {
    Retry(Duration),
    Fail(ErrorKind),
}

pub fn after_failure(
    kind: ErrorKind,
    retry_after: Option<Duration>,
    attempts: i64,
    remaining: Duration,
    limits: &Limits,
) -> NextStep {
    if !kind.is_transient() || attempts >= MAX_ATTEMPTS {
        return NextStep::Fail(kind);
    }
    let wait = match retry_after {
        Some(wait) if wait > limits.max_retry_after => {
            return NextStep::Fail(ErrorKind::RateLimited);
        }
        Some(wait) => wait,
        None => limits.backoff[(attempts - 1).clamp(0, 1) as usize],
    };
    if wait >= remaining {
        NextStep::Fail(ErrorKind::Timeout)
    } else {
        NextStep::Retry(wait)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLENTY: Duration = Duration::from_secs(600);

    #[test]
    fn error_kinds_round_trip_and_match_the_schema() {
        for kind in ERROR_KINDS {
            assert_eq!(ErrorKind::parse(kind.as_str()), Some(kind));
            assert_eq!(
                serde_json::to_string(&kind).unwrap(),
                format!("\"{}\"", kind.as_str())
            );
        }
        let schema =
            include_str!("../../migrations/20261130000002_rebuild_agent_runs_for_tool_budgets.sql");
        for kind in ERROR_KINDS {
            assert!(schema.contains(&format!("'{}'", kind.as_str())));
        }
        assert_eq!(ErrorKind::parse("exploded"), None);
    }

    #[test]
    fn only_a_reply_that_is_just_pass_is_a_pass() {
        for reply in ["PASS", "pass", " Pass. ", "\"PASS\"", "**PASS**"] {
            assert!(is_pass(reply), "{reply}");
        }
        for reply in ["", "I will pass on this", "PASS, but one thing", "Passing"] {
            assert!(!is_pass(reply), "{reply}");
        }
    }

    #[test]
    fn parses_every_status() {
        for (text, status) in [
            ("queued", RunStatus::Queued),
            ("running", RunStatus::Running),
            ("succeeded", RunStatus::Succeeded),
            ("passed", RunStatus::Passed),
            ("failed", RunStatus::Failed),
        ] {
            assert_eq!(RunStatus::parse(text), Some(status));
        }
        assert_eq!(RunStatus::parse("cancelled"), None);
    }

    #[test]
    fn only_rate_limits_timeouts_and_unavailability_are_transient() {
        let transient: Vec<ErrorKind> = ERROR_KINDS
            .into_iter()
            .filter(|kind| kind.is_transient())
            .collect();

        assert_eq!(
            transient,
            [
                ErrorKind::RateLimited,
                ErrorKind::Timeout,
                ErrorKind::ProviderUnavailable
            ]
        );
    }

    #[test]
    fn accepts_only_canonical_lowercase_uuids() {
        assert!(is_canonical_client_request_id(
            "0196f3a0-0000-7000-8000-000000000001"
        ));
        assert!(!is_canonical_client_request_id(
            "0196F3A0-0000-7000-8000-000000000001"
        ));
        assert!(!is_canonical_client_request_id(
            "0196f3a0000070008000000000000001"
        ));
        assert!(!is_canonical_client_request_id(
            "{0196f3a0-0000-7000-8000-000000000001}"
        ));
        assert!(!is_canonical_client_request_id(""));
    }

    #[test]
    fn asks_for_the_smaller_of_the_model_maximum_and_the_internal_cap() {
        let limits = Limits::default();

        assert_eq!(limits.output_tokens(Location::Cloud, Some(128_000)), 16_000);
        assert_eq!(limits.output_tokens(Location::Cloud, Some(8_000)), 8_000);
        assert_eq!(limits.output_tokens(Location::Local, None), 2048);
        assert_eq!(limits.max_response_bytes, 262_144);
    }

    #[test]
    fn retries_transient_errors_with_growing_backoff() {
        let limits = Limits::default();

        assert_eq!(
            after_failure(ErrorKind::ProviderUnavailable, None, 1, PLENTY, &limits),
            NextStep::Retry(Duration::from_secs(2))
        );
        assert_eq!(
            after_failure(ErrorKind::Timeout, None, 2, PLENTY, &limits),
            NextStep::Retry(Duration::from_secs(8))
        );
        assert_eq!(
            after_failure(ErrorKind::RateLimited, None, 3, PLENTY, &limits),
            NextStep::Fail(ErrorKind::RateLimited)
        );
    }

    #[test]
    fn fails_permanent_errors_at_once() {
        assert_eq!(
            after_failure(
                ErrorKind::ProviderAuthFailed,
                None,
                1,
                PLENTY,
                &Limits::default()
            ),
            NextStep::Fail(ErrorKind::ProviderAuthFailed)
        );
    }

    #[test]
    fn honours_retry_after_up_to_the_maximum() {
        let limits = Limits::default();

        assert_eq!(
            after_failure(
                ErrorKind::RateLimited,
                Some(Duration::from_secs(30)),
                1,
                PLENTY,
                &limits
            ),
            NextStep::Retry(Duration::from_secs(30))
        );
        assert_eq!(
            after_failure(
                ErrorKind::RateLimited,
                Some(Duration::from_secs(61)),
                1,
                PLENTY,
                &limits
            ),
            NextStep::Fail(ErrorKind::RateLimited)
        );
    }

    #[test]
    fn a_wait_that_does_not_fit_the_remaining_time_ends_in_timeout() {
        assert_eq!(
            after_failure(
                ErrorKind::ProviderUnavailable,
                None,
                1,
                Duration::from_secs(1),
                &Limits::default()
            ),
            NextStep::Fail(ErrorKind::Timeout)
        );
    }
}
