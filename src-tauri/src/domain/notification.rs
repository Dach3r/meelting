use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationSetting {
    RunSucceeded,
    RunFailed,
    Sound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_from_snake_case_and_rejects_unknown_input() {
        assert_eq!(
            serde_json::from_str::<NotificationSetting>("\"run_failed\"").unwrap(),
            NotificationSetting::RunFailed
        );
        assert!(serde_json::from_str::<NotificationSetting>("\"badge\"").is_err());
    }
}
