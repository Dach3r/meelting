use serde::{Deserialize, Serialize};

pub const TITLE_MAX_CHARS: usize = 120;
pub const DESCRIPTION_MAX_CHARS: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Blocked,
    Completed,
    Canceled,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Blocked => "blocked",
            TaskStatus::Completed => "completed",
            TaskStatus::Canceled => "canceled",
        }
    }

    pub fn parse(value: &str) -> Option<TaskStatus> {
        match value {
            "pending" => Some(TaskStatus::Pending),
            "in_progress" => Some(TaskStatus::InProgress),
            "blocked" => Some(TaskStatus::Blocked),
            "completed" => Some(TaskStatus::Completed),
            "canceled" => Some(TaskStatus::Canceled),
            _ => None,
        }
    }

    pub fn can_change_to(self, next: TaskStatus) -> bool {
        use TaskStatus::*;
        matches!(
            (self, next),
            (Pending, InProgress | Blocked | Completed | Canceled)
                | (InProgress, Pending | Blocked | Completed | Canceled)
                | (Blocked, Pending | InProgress | Canceled)
                | (Completed, InProgress)
                | (Canceled, Pending)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
}

impl TaskPriority {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskPriority::Low => "low",
            TaskPriority::Medium => "medium",
            TaskPriority::High => "high",
        }
    }

    pub fn parse(value: &str) -> Option<TaskPriority> {
        match value {
            "low" => Some(TaskPriority::Low),
            "medium" => Some(TaskPriority::Medium),
            "high" => Some(TaskPriority::High),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub repository_id: Option<String>,
    pub base_ref: Option<String>,
    pub origin_room_id: Option<String>,
    pub origin_room_name: Option<String>,
    pub origin_message_id: Option<String>,
    pub linked_rooms: Vec<TaskRoomLink>,
    pub agent_ids: Vec<String>,
    pub worktree: Option<crate::domain::worktree::Worktree>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TaskDraft {
    pub title: String,
    pub description: String,
    pub priority: TaskPriority,
    pub repository_id: Option<String>,
    pub base_ref: Option<String>,
    pub agent_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskRoomLink {
    pub room_id: String,
    pub room_name: String,
    pub message_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TaskOrigin {
    pub room_id: String,
    pub message_id: Option<String>,
}

impl TaskDraft {
    pub fn normalize(self) -> Option<Self> {
        let title = self.title.trim().to_string();
        let description = self.description.trim().to_string();
        let repository_id = self.repository_id.filter(|id| !id.is_empty());
        let base_ref = self
            .base_ref
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty());
        let valid = (1..=TITLE_MAX_CHARS).contains(&title.chars().count())
            && description.chars().count() <= DESCRIPTION_MAX_CHARS
            && (base_ref.is_none() || repository_id.is_some());
        valid.then(|| Self {
            title,
            description,
            priority: self.priority,
            repository_id,
            base_ref,
            agent_ids: crate::domain::room::unique_ids(self.agent_ids),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> TaskDraft {
        TaskDraft {
            title: "  Fix login  ".into(),
            description: "  Steps \n".into(),
            priority: TaskPriority::High,
            repository_id: Some("r1".into()),
            base_ref: Some(" main ".into()),
            agent_ids: ["a", "b", "a"].map(String::from).to_vec(),
        }
    }

    #[test]
    fn statuses_and_priorities_round_trip_through_text() {
        use TaskStatus::*;
        for status in [Pending, InProgress, Blocked, Completed, Canceled] {
            assert_eq!(TaskStatus::parse(status.as_str()), Some(status));
        }
        for priority in [TaskPriority::Low, TaskPriority::Medium, TaskPriority::High] {
            assert_eq!(TaskPriority::parse(priority.as_str()), Some(priority));
        }
        assert_eq!(TaskStatus::parse("done"), None);
        assert_eq!(TaskPriority::parse("urgent"), None);
    }

    #[test]
    fn serializes_like_the_database() {
        assert_eq!(
            serde_json::to_string(&TaskStatus::InProgress).unwrap(),
            "\"in_progress\""
        );
        assert_eq!(
            serde_json::to_string(&TaskPriority::Medium).unwrap(),
            "\"medium\""
        );
    }

    #[test]
    fn allows_only_the_approved_transitions() {
        use TaskStatus::*;
        let all = [Pending, InProgress, Blocked, Completed, Canceled];
        let allowed = [
            (Pending, InProgress),
            (Pending, Blocked),
            (Pending, Completed),
            (Pending, Canceled),
            (InProgress, Pending),
            (InProgress, Blocked),
            (InProgress, Completed),
            (InProgress, Canceled),
            (Blocked, Pending),
            (Blocked, InProgress),
            (Blocked, Canceled),
            (Completed, InProgress),
            (Canceled, Pending),
        ];
        for from in all {
            for to in all {
                assert_eq!(
                    from.can_change_to(to),
                    allowed.contains(&(from, to)),
                    "{from:?} -> {to:?}"
                );
            }
        }
    }

    #[test]
    fn normalizes_the_draft() {
        let draft = draft().normalize().unwrap();

        assert_eq!(draft.title, "Fix login");
        assert_eq!(draft.description, "Steps");
        assert_eq!(draft.base_ref.as_deref(), Some("main"));
        assert_eq!(draft.agent_ids, ["a", "b"]);
    }

    #[test]
    fn empty_optional_values_become_none() {
        let draft = TaskDraft {
            repository_id: Some(String::new()),
            base_ref: Some("  ".into()),
            ..draft()
        }
        .normalize()
        .unwrap();

        assert_eq!((draft.repository_id, draft.base_ref), (None, None));
    }

    #[test]
    fn rejects_invalid_titles_descriptions_and_a_base_without_repository() {
        for invalid in [
            TaskDraft {
                title: "   ".into(),
                ..draft()
            },
            TaskDraft {
                title: "ñ".repeat(TITLE_MAX_CHARS + 1),
                ..draft()
            },
            TaskDraft {
                description: "a".repeat(DESCRIPTION_MAX_CHARS + 1),
                ..draft()
            },
            TaskDraft {
                repository_id: None,
                ..draft()
            },
        ] {
            assert_eq!(invalid.normalize(), None);
        }
    }

    #[test]
    fn accepts_text_at_the_limits() {
        let draft = TaskDraft {
            title: "ñ".repeat(TITLE_MAX_CHARS),
            description: "ñ".repeat(DESCRIPTION_MAX_CHARS),
            ..draft()
        };

        assert!(draft.normalize().is_some());
    }
}
