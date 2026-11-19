use std::path::{Path, PathBuf};
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

pub async fn pick_files(window: &WebviewWindow, folder: &Path) -> Option<Vec<PathBuf>> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    window
        .dialog()
        .file()
        .set_parent(window)
        .set_directory(folder)
        .pick_files(move |files| {
            let _ = sender.send(files);
        });
    receiver
        .await
        .ok()
        .flatten()?
        .into_iter()
        .map(|file| file.into_path().ok())
        .collect()
}
