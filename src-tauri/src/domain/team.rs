use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Agent {
    pub id: String,
    pub team_id: String,
    pub name: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub agents: Vec<Agent>,
}

pub const TEXT_MAX_CHARS: usize = 80;
pub const DESCRIPTION_MAX_CHARS: usize = 500;
pub const INSTRUCTIONS_MAX_CHARS: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AgentDraft {
    pub team_id: String,
    pub name: String,
    pub role: String,
    pub description: String,
    pub instructions: String,
    pub provider: String,
    pub model: String,
}

impl AgentDraft {
    pub fn normalize(self) -> Option<Self> {
        let draft = Self {
            team_id: self.team_id,
            name: self.name.trim().to_string(),
            role: self.role.trim().to_string(),
            description: self.description.trim().to_string(),
            instructions: self.instructions.trim().to_string(),
            provider: self.provider.trim().to_string(),
            model: self.model.trim().to_string(),
        };
        let valid = (1..=TEXT_MAX_CHARS).contains(&draft.name.chars().count())
            && (1..=TEXT_MAX_CHARS).contains(&draft.role.chars().count())
            && draft.description.chars().count() <= DESCRIPTION_MAX_CHARS
            && draft.instructions.chars().count() <= INSTRUCTIONS_MAX_CHARS
            && !draft.provider.is_empty()
            && !draft.model.is_empty();
        valid.then_some(draft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> AgentDraft {
        AgentDraft {
            team_id: "t1".into(),
            name: "  Ana ".into(),
            role: "Developer".into(),
            description: String::new(),
            instructions: String::new(),
            provider: "openai".into(),
            model: "gpt".into(),
        }
    }

    #[test]
    fn trims_every_text_field() {
        assert_eq!(draft().normalize().unwrap().name, "Ana");
    }

    #[test]
    fn allows_empty_description_and_instructions() {
        assert!(draft().normalize().is_some());
    }

    #[test]
    fn rejects_blank_required_fields() {
        for blank in ["name", "role", "provider", "model"] {
            let mut invalid = draft();
            match blank {
                "name" => invalid.name = "  ".into(),
                "role" => invalid.role = "".into(),
                "provider" => invalid.provider = " ".into(),
                _ => invalid.model = "".into(),
            }
            assert_eq!(invalid.normalize(), None, "{blank}");
        }
    }

    #[test]
    fn rejects_fields_over_their_limit() {
        let mut long_name = draft();
        long_name.name = "a".repeat(TEXT_MAX_CHARS + 1);
        let mut long_description = draft();
        long_description.description = "a".repeat(DESCRIPTION_MAX_CHARS + 1);
        let mut long_instructions = draft();
        long_instructions.instructions = "a".repeat(INSTRUCTIONS_MAX_CHARS + 1);

        assert_eq!(long_name.normalize(), None);
        assert_eq!(long_description.normalize(), None);
        assert_eq!(long_instructions.normalize(), None);
    }

    #[test]
    fn accepts_exactly_the_limits() {
        let mut limits = draft();
        limits.role = "ñ".repeat(TEXT_MAX_CHARS);
        limits.description = "ñ".repeat(DESCRIPTION_MAX_CHARS);
        limits.instructions = "ñ".repeat(INSTRUCTIONS_MAX_CHARS);

        assert!(limits.normalize().is_some());
    }
}
