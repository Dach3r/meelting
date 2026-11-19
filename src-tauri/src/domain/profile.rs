use crate::domain::locale::Locale;
use crate::domain::reduce_motion::ReduceMotion;
use crate::domain::theme::Theme;
use crate::domain::time_format::TimeFormat;
use crate::domain::ui_scale::UiScale;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub locale: Locale,
    pub theme: Theme,
    pub ui_scale: UiScale,
    pub loading_enabled: bool,
    pub time_format: TimeFormat,
    pub reduce_motion: ReduceMotion,
    pub notify_run_succeeded: bool,
    pub notify_run_failed: bool,
    pub notify_sound: bool,
    pub current_workspace_id: Option<String>,
}
