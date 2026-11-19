use crate::domain::agent_run::{AgentRun, Partial};
use serde::Serialize;

pub const BODY_MAX_CHARS: usize = 4000;
pub const RECENT_MESSAGES: i64 = 100;
pub const PREVIEW_MESSAGES: i64 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Participant {
    pub id: String,
    pub name: String,
    pub role: String,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub updated_at: String,
    pub finished_at: Option<String>,
    pub agents: Vec<Participant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Roster {
    pub agents: Vec<Participant>,
    pub former_agents: Vec<Participant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Message {
    pub id: String,
    pub room_id: String,
    pub agent_id: Option<String>,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoomList {
    pub rooms: Vec<Room>,
    pub featured_messages: Vec<Message>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoomDetail {
    pub room: Room,
    pub former_agents: Vec<Participant>,
    pub messages: Vec<Message>,
    pub runs: Vec<AgentRun>,
    pub partials: Vec<Partial>,
    pub proposed_agent_id: Option<String>,
}

pub fn normalize_body(raw: &str) -> Option<String> {
    let body = raw.trim();
    let length = body.chars().count();
    (1..=BODY_MAX_CHARS)
        .contains(&length)
        .then(|| body.to_string())
}

pub fn unique_ids(ids: Vec<String>) -> Vec<String> {
    let mut unique: Vec<String> = Vec::with_capacity(ids.len());
    for id in ids {
        if !unique.contains(&id) {
            unique.push(id);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_the_body() {
        assert_eq!(normalize_body("  hola \n"), Some("hola".to_string()));
    }

    #[test]
    fn rejects_blank_bodies() {
        assert_eq!(normalize_body(""), None);
        assert_eq!(normalize_body(" \n\t "), None);
    }

    #[test]
    fn accepts_exactly_the_maximum_length_in_characters() {
        assert!(normalize_body(&"ñ".repeat(BODY_MAX_CHARS)).is_some());
    }

    #[test]
    fn rejects_bodies_over_the_maximum_length() {
        assert_eq!(normalize_body(&"a".repeat(BODY_MAX_CHARS + 1)), None);
    }

    #[test]
    fn keeps_the_first_appearance_of_each_id() {
        let ids = ["b", "a", "b", "c", "a"].map(String::from).to_vec();

        assert_eq!(unique_ids(ids), ["b", "a", "c"]);
    }
}
