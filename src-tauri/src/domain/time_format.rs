use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeFormat {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "12h")]
    H12,
    #[serde(rename = "24h")]
    H24,
}

impl TimeFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            TimeFormat::Auto => "auto",
            TimeFormat::H12 => "12h",
            TimeFormat::H24 => "24h",
        }
    }

    pub fn parse(value: &str) -> Option<TimeFormat> {
        match value {
            "auto" => Some(TimeFormat::Auto),
            "12h" => Some(TimeFormat::H12),
            "24h" => Some(TimeFormat::H24),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_text() {
        for format in [TimeFormat::Auto, TimeFormat::H12, TimeFormat::H24] {
            assert_eq!(TimeFormat::parse(format.as_str()), Some(format));
        }
    }

    #[test]
    fn rejects_unknown_format() {
        assert_eq!(TimeFormat::parse("military"), None);
    }

    #[test]
    fn serializes_as_the_stored_text_and_rejects_invalid_input() {
        assert_eq!(serde_json::to_string(&TimeFormat::H24).unwrap(), "\"24h\"");
        assert!(serde_json::from_str::<TimeFormat>("\"military\"").is_err());
    }
}
