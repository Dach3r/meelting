use crate::services::voice::VoiceStatus;

#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn get_voice_status() -> VoiceStatus {
    crate::services::voice::unsupported()
}

#[cfg(target_os = "macos")]
mod macos {
    use super::VoiceStatus;
    use crate::services::session_load::SessionError;
    use crate::services::voice::{EmitPlayback, Voice};
    use sqlx::SqlitePool;
    use std::sync::Arc;
    use tauri::{AppHandle, Emitter, State};

    pub fn playback_emitter(app: AppHandle) -> EmitPlayback {
        Arc::new(move |playback| {
            let _ = app.emit("voice:playback", playback);
        })
    }

    #[tauri::command]
    pub fn get_voice_status(voice: State<'_, Arc<Voice>>) -> VoiceStatus {
        voice.status()
    }

    #[tauri::command]
    pub async fn download_voice_model(
        app: AppHandle,
        voice: State<'_, Arc<Voice>>,
    ) -> Result<VoiceStatus, SessionError> {
        voice
            .download_model(|downloaded| {
                let _ = app.emit("voice:download_progress", downloaded);
            })
            .await
    }

    #[tauri::command]
    pub async fn start_voice_capture(voice: State<'_, Arc<Voice>>) -> Result<(), SessionError> {
        let voice = voice.inner().clone();
        tokio::task::spawn_blocking(move || voice.start_capture())
            .await
            .map_err(|_| SessionError::MicrophoneFailed)?
    }

    #[tauri::command]
    pub async fn finish_voice_capture(
        voice: State<'_, Arc<Voice>>,
        pool: State<'_, SqlitePool>,
    ) -> Result<String, SessionError> {
        voice.finish_capture(&pool).await
    }

    #[tauri::command]
    pub async fn cancel_voice_capture(voice: State<'_, Arc<Voice>>) -> Result<(), ()> {
        let voice = voice.inner().clone();
        let _ = tokio::task::spawn_blocking(move || voice.cancel_capture()).await;
        Ok(())
    }

    #[tauri::command]
    pub async fn speak_message(
        voice: State<'_, Arc<Voice>>,
        pool: State<'_, SqlitePool>,
        room_id: String,
        message_id: String,
    ) -> Result<(), SessionError> {
        voice
            .inner()
            .clone()
            .speak(&pool, &room_id, &message_id)
            .await
    }

    #[tauri::command]
    pub fn stop_speaking(voice: State<'_, Arc<Voice>>) {
        voice.stop_speaking();
    }
}
