use crate::domain::locale::Locale;
use crate::domain::theme::Theme;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub locale: Locale,
    pub theme: Theme,
    pub current_workspace_id: Option<String>,
}
