use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    Es,
    En,
}

impl Locale {
    pub fn as_str(self) -> &'static str {
        match self {
            Locale::Es => "es",
            Locale::En => "en",
        }
    }

    pub fn parse(value: &str) -> Option<Locale> {
        match value {
            "es" => Some(Locale::Es),
            "en" => Some(Locale::En),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_text() {
        for locale in [Locale::Es, Locale::En] {
            assert_eq!(Locale::parse(locale.as_str()), Some(locale));
        }
    }

    #[test]
    fn rejects_unknown_locale() {
        assert_eq!(Locale::parse("fr"), None);
    }

    #[test]
    fn serializes_lowercase() {
        assert_eq!(serde_json::to_string(&Locale::Es).unwrap(), "\"es\"");
    }
}
