use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Agent {
    pub id: String,
    pub team_id: String,
    pub name: String,
    pub role: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentDetail {
    pub id: String,
    pub team_id: String,
    pub template_key: Option<String>,
    pub name: String,
    pub role: String,
    pub description: String,
    pub instructions: String,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub agents: Vec<Agent>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Position {
    pub node_id: String,
    pub x: f64,
    pub y: f64,
}

pub const TEXT_MAX_CHARS: usize = 80;
pub const DESCRIPTION_MAX_CHARS: usize = 500;
pub const INSTRUCTIONS_MAX_CHARS: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AgentDraft {
    pub team_id: String,
    pub template_key: Option<String>,
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
            template_key: self.template_key.filter(|key| !key.trim().is_empty()),
            name: self.name.trim().to_string(),
            role: self.role.trim().to_string(),
            description: self.description.trim().to_string(),
            instructions: self.instructions.trim().to_string(),
            provider: self.provider.trim().to_string(),
            model: self.model.trim().to_string(),
        };
        let valid = identity_is_valid(
            &draft.name,
            &draft.role,
            &draft.description,
            &draft.instructions,
        ) && !draft.provider.is_empty()
            && !draft.model.is_empty();
        valid.then_some(draft)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AgentEdit {
    pub team_id: String,
    pub name: String,
    pub role: String,
    pub description: String,
    pub instructions: String,
    pub provider: String,
    pub model: String,
}

impl AgentEdit {
    pub fn normalize(self) -> Option<Self> {
        let edit = Self {
            team_id: self.team_id,
            name: self.name.trim().to_string(),
            role: self.role.trim().to_string(),
            description: self.description.trim().to_string(),
            instructions: self.instructions.trim().to_string(),
            provider: self.provider.trim().to_string(),
            model: self.model.trim().to_string(),
        };
        let valid = identity_is_valid(
            &edit.name,
            &edit.role,
            &edit.description,
            &edit.instructions,
        ) && !edit.provider.is_empty()
            && !edit.model.is_empty();
        valid.then_some(edit)
    }
}

fn identity_is_valid(name: &str, role: &str, description: &str, instructions: &str) -> bool {
    (1..=TEXT_MAX_CHARS).contains(&name.chars().count())
        && (1..=TEXT_MAX_CHARS).contains(&role.chars().count())
        && description.chars().count() <= DESCRIPTION_MAX_CHARS
        && instructions.chars().count() <= INSTRUCTIONS_MAX_CHARS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> AgentDraft {
        AgentDraft {
            team_id: "t1".into(),
            template_key: None,
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
    fn drops_a_blank_template_key() {
        let mut blank = draft();
        blank.template_key = Some("  ".into());
        let mut named = draft();
        named.template_key = Some("product_owner".into());

        assert_eq!(blank.normalize().unwrap().template_key, None);
        assert_eq!(
            named.normalize().unwrap().template_key.as_deref(),
            Some("product_owner")
        );
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
    fn normalizes_an_edit_with_the_same_limits() {
        let edit = |name: &str, description: String| AgentEdit {
            team_id: "t1".into(),
            name: name.into(),
            role: "Developer".into(),
            description,
            instructions: String::new(),
            provider: " ollama ".into(),
            model: "qwen".into(),
        };

        assert_eq!(
            edit("  Ana ", String::new()).normalize().unwrap().name,
            "Ana"
        );
        assert_eq!(
            edit("Ana", String::new()).normalize().unwrap().provider,
            "ollama"
        );
        assert_eq!(edit(" ", String::new()).normalize(), None);
        let mut blank_model = edit("Ana", String::new());
        blank_model.model = "  ".into();
        assert_eq!(blank_model.normalize(), None);
        assert_eq!(
            edit("Ana", "a".repeat(DESCRIPTION_MAX_CHARS + 1)).normalize(),
            None
        );
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
