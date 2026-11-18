use crate::domain::agent_run::ErrorKind;
use std::time::Duration;

pub fn from_status(status: u16, code: Option<&str>) -> ErrorKind {
    match status {
        401 | 403 => ErrorKind::ProviderAuthFailed,
        402 => ErrorKind::ProviderBilling,
        404 => ErrorKind::ModelUnavailable,
        408 => ErrorKind::Timeout,
        429 if code.is_some_and(is_billing_code) => ErrorKind::ProviderBilling,
        429 => ErrorKind::RateLimited,
        500..=599 => ErrorKind::ProviderUnavailable,
        _ => ErrorKind::ProviderRejected,
    }
}

pub fn from_code(code: &str) -> ErrorKind {
    match code {
        "overloaded_error"
        | "api_error"
        | "server_error"
        | "internal_error"
        | "service_unavailable" => ErrorKind::ProviderUnavailable,
        "rate_limit_error" | "rate_limit_exceeded" => ErrorKind::RateLimited,
        "authentication_error" | "permission_error" | "invalid_api_key" => {
            ErrorKind::ProviderAuthFailed
        }
        "not_found_error" | "model_not_found" => ErrorKind::ModelUnavailable,
        "timeout_error" => ErrorKind::Timeout,
        code if is_billing_code(code) => ErrorKind::ProviderBilling,
        _ => ErrorKind::ProviderRejected,
    }
}

fn is_billing_code(code: &str) -> bool {
    matches!(code, "insufficient_quota" | "billing_error")
}

pub fn retry_after(header: &str) -> Option<Duration> {
    let seconds: f64 = header.trim().parse().ok()?;
    (seconds.is_finite() && seconds >= 0.0).then(|| Duration::from_secs_f64(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_http_statuses() {
        for (status, kind) in [
            (401, ErrorKind::ProviderAuthFailed),
            (403, ErrorKind::ProviderAuthFailed),
            (402, ErrorKind::ProviderBilling),
            (404, ErrorKind::ModelUnavailable),
            (408, ErrorKind::Timeout),
            (429, ErrorKind::RateLimited),
            (500, ErrorKind::ProviderUnavailable),
            (503, ErrorKind::ProviderUnavailable),
            (529, ErrorKind::ProviderUnavailable),
            (400, ErrorKind::ProviderRejected),
            (413, ErrorKind::ProviderRejected),
        ] {
            assert_eq!(from_status(status, None), kind, "{status}");
        }
    }

    #[test]
    fn an_exhausted_quota_is_billing_not_a_rate_limit() {
        assert_eq!(
            from_status(429, Some("insufficient_quota")),
            ErrorKind::ProviderBilling
        );
        assert_eq!(
            from_status(429, Some("rate_limit_exceeded")),
            ErrorKind::RateLimited
        );
    }

    #[test]
    fn maps_stream_error_codes() {
        assert_eq!(
            from_code("overloaded_error"),
            ErrorKind::ProviderUnavailable
        );
        assert_eq!(from_code("rate_limit_error"), ErrorKind::RateLimited);
        assert_eq!(
            from_code("authentication_error"),
            ErrorKind::ProviderAuthFailed
        );
        assert_eq!(from_code("billing_error"), ErrorKind::ProviderBilling);
        assert_eq!(from_code("model_not_found"), ErrorKind::ModelUnavailable);
        assert_eq!(
            from_code("invalid_request_error"),
            ErrorKind::ProviderRejected
        );
    }

    #[test]
    fn reads_retry_after_in_seconds_only() {
        assert_eq!(retry_after("7"), Some(Duration::from_secs(7)));
        assert_eq!(retry_after(" 1.5 "), Some(Duration::from_millis(1500)));
        assert_eq!(retry_after("Wed, 21 Oct 2026 07:28:00 GMT"), None);
        assert_eq!(retry_after("-1"), None);
    }
}
