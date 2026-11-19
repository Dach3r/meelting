use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct UiScale(u16);

const PRESETS: [u16; 5] = [90, 100, 110, 125, 150];

impl UiScale {
    pub fn percent(self) -> u16 {
        self.0
    }

    pub fn from_percent(value: u16) -> Option<UiScale> {
        PRESETS.contains(&value).then_some(UiScale(value))
    }
}

impl Default for UiScale {
    fn default() -> Self {
        UiScale(100)
    }
}

impl TryFrom<u16> for UiScale {
    type Error = String;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        UiScale::from_percent(value).ok_or_else(|| format!("invalid ui scale: {value}"))
    }
}

impl From<UiScale> for u16 {
    fn from(scale: UiScale) -> u16 {
        scale.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_every_preset() {
        for value in PRESETS {
            assert_eq!(UiScale::from_percent(value).unwrap().percent(), value);
        }
    }

    #[test]
    fn rejects_unknown_scale() {
        assert_eq!(UiScale::from_percent(105), None);
    }

    #[test]
    fn defaults_to_one_hundred() {
        assert_eq!(UiScale::default().percent(), 100);
    }

    #[test]
    fn serializes_as_a_number_and_rejects_invalid_input() {
        assert_eq!(
            serde_json::to_string(&UiScale::from_percent(125).unwrap()).unwrap(),
            "125"
        );
        assert!(serde_json::from_str::<UiScale>("105").is_err());
    }
}
