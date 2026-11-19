use std::path::PathBuf;
use tauri::WebviewWindow;
use tauri_plugin_dialog::DialogExt;

pub async fn pick_folder(window: &WebviewWindow) -> Option<PathBuf> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    window
        .dialog()
        .file()
        .set_parent(window)
        .pick_folder(move |folder| {
            let _ = sender.send(folder);
        });
    receiver.await.ok().flatten()?.into_path().ok()
}
