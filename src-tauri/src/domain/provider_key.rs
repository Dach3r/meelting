use crate::domain::agent_run::ErrorKind;
use serde::Serialize;

const MAX_KEY_CHARS: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Verification {
    Verified,
    Rejected { reason: ErrorKind },
    Offline,
    ProviderBusy,
}

pub fn secret_name(workspace_id: &str, provider: &str) -> String {
    format!("{workspace_id}/{provider}")
}

pub fn normalize(key: &str) -> Option<&str> {
    let key = key.trim();
    let valid = !key.is_empty()
        && key.chars().count() <= MAX_KEY_CHARS
        && !key.chars().any(|c| c.is_whitespace() || c.is_control());
    valid.then_some(key)
}

pub fn from_failure(kind: ErrorKind, unreachable: bool) -> Verification {
    match kind {
        _ if unreachable => Verification::Offline,
        ErrorKind::OutputTruncated | ErrorKind::ContentFiltered => Verification::Verified,
        ErrorKind::ProviderUnavailable
        | ErrorKind::RateLimited
        | ErrorKind::Timeout
        | ErrorKind::InvalidResponse => Verification::ProviderBusy,
        reason => Verification::Rejected { reason },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_secret_by_workspace_and_provider() {
        assert_eq!(secret_name("ws-1", "groq"), "ws-1/groq");
    }

    #[test]
    fn trims_the_key_and_rejects_empty_spaced_control_or_oversized_keys() {
        assert_eq!(normalize("  sk-abc123\n"), Some("sk-abc123"));
        assert_eq!(normalize(""), None);
        assert_eq!(normalize("   "), None);
        assert_eq!(normalize("sk abc"), None);
        assert_eq!(normalize("sk-\u{7}abc"), None);
        assert_eq!(normalize(&"k".repeat(512)).map(str::len), Some(512));
        assert_eq!(normalize(&"k".repeat(513)), None);
    }

    #[test]
    fn a_reply_cut_by_the_token_cap_or_filtered_still_proves_the_key() {
        assert_eq!(
            from_failure(ErrorKind::OutputTruncated, false),
            Verification::Verified
        );
        assert_eq!(
            from_failure(ErrorKind::ContentFiltered, false),
            Verification::Verified
        );
    }

    #[test]
    fn a_connection_that_never_happened_is_offline_not_a_bad_key() {
        assert_eq!(
            from_failure(ErrorKind::ProviderUnavailable, true),
            Verification::Offline
        );
    }

    #[test]
    fn transient_provider_failures_cannot_verify_the_key() {
        for kind in [
            ErrorKind::ProviderUnavailable,
            ErrorKind::RateLimited,
            ErrorKind::Timeout,
            ErrorKind::InvalidResponse,
        ] {
            assert_eq!(from_failure(kind, false), Verification::ProviderBusy);
        }
    }

    #[test]
    fn auth_billing_and_access_failures_reject_the_key() {
        for kind in [
            ErrorKind::ProviderAuthFailed,
            ErrorKind::ProviderBilling,
            ErrorKind::ModelUnavailable,
            ErrorKind::ProviderRejected,
        ] {
            assert_eq!(
                from_failure(kind, false),
                Verification::Rejected { reason: kind }
            );
        }
    }

    #[test]
    fn serializes_as_a_status_with_an_optional_reason() {
        assert_eq!(
            serde_json::to_value(Verification::Rejected {
                reason: ErrorKind::ProviderAuthFailed
            })
            .unwrap(),
            serde_json::json!({ "status": "rejected", "reason": "provider_auth_failed" })
        );
        assert_eq!(
            serde_json::to_value(Verification::Offline).unwrap(),
            serde_json::json!({ "status": "offline" })
        );
    }
}
