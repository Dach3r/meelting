use crate::services::session_load::SessionError;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub fn get_open_at_login(app: AppHandle) -> Result<bool, SessionError> {
    app.autolaunch()
        .is_enabled()
        .map_err(|_| SessionError::AutostartUnavailable)
}

#[tauri::command]
pub fn change_open_at_login(app: AppHandle, enabled: bool) -> Result<bool, SessionError> {
    let launcher = app.autolaunch();
    let changed = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    changed.map_err(|_| SessionError::AutostartUnavailable)?;
    launcher
        .is_enabled()
        .map_err(|_| SessionError::AutostartUnavailable)
}

#[tauri::command]
pub fn open_data_folder(app: AppHandle) -> Result<(), SessionError> {
    let folder = app
        .path()
        .app_data_dir()
        .map_err(|_| SessionError::DataFolderUnavailable)?;
    app.opener()
        .open_path(folder.to_string_lossy(), None::<&str>)
        .map_err(|_| SessionError::DataFolderUnavailable)
}
