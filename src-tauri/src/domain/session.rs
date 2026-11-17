use crate::domain::locale::Locale;
use crate::domain::theme::Theme;
use crate::domain::workspace::Workspace;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Session {
    pub locale: Locale,
    pub theme: Theme,
    pub current_workspace_id: String,
    pub workspaces: Vec<Workspace>,
}
