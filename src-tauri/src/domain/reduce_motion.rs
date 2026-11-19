use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReduceMotion {
    System,
    On,
    Off,
}

impl ReduceMotion {
    pub fn as_str(self) -> &'static str {
        match self {
            ReduceMotion::System => "system",
            ReduceMotion::On => "on",
            ReduceMotion::Off => "off",
        }
    }

    pub fn parse(value: &str) -> Option<ReduceMotion> {
        match value {
            "system" => Some(ReduceMotion::System),
            "on" => Some(ReduceMotion::On),
            "off" => Some(ReduceMotion::Off),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_text() {
        for value in [ReduceMotion::System, ReduceMotion::On, ReduceMotion::Off] {
            assert_eq!(ReduceMotion::parse(value.as_str()), Some(value));
        }
    }

    #[test]
    fn rejects_unknown_value() {
        assert_eq!(ReduceMotion::parse("some"), None);
    }

    #[test]
    fn serializes_lowercase() {
        assert_eq!(serde_json::to_string(&ReduceMotion::On).unwrap(), "\"on\"");
    }
}
