use crate::domain::locale::Locale;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub locale: Locale,
    pub current_workspace_id: Option<String>,
}
