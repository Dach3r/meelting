use crate::domain::locale::Locale;
use crate::domain::theme::Theme;
use crate::domain::ui_scale::UiScale;
use crate::domain::workspace::Workspace;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Session {
    pub locale: Locale,
    pub theme: Theme,
    pub ui_scale: UiScale,
    pub loading_enabled: bool,
    pub current_workspace_id: String,
    pub workspaces: Vec<Workspace>,
}
