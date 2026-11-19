use crate::domain::locale::Locale;
use crate::domain::theme::Theme;
use crate::domain::ui_scale::UiScale;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub locale: Locale,
    pub theme: Theme,
    pub ui_scale: UiScale,
    pub loading_enabled: bool,
    pub current_workspace_id: Option<String>,
}
