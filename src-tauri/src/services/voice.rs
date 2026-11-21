use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct VoiceStatus {
    pub available: bool,
    pub model_ready: bool,
    pub model_bytes: u64,
}

#[cfg(not(target_os = "macos"))]
pub fn unsupported() -> VoiceStatus {
    VoiceStatus {
        available: false,
        model_ready: false,
        model_bytes: 0,
    }
}

#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(target_os = "macos")]
mod macos {
    use super::VoiceStatus;
    use crate::domain::locale::Locale;
    use crate::domain::speech::{self, Heard, RECOGNITION_RATE, SystemVoice};
    use crate::repositories::messages;
    use crate::services::session_load::{self, SessionError};
    use crate::system::audio_capture::Capture;
    use crate::system::{speech_recognition, speech_synthesis};
    use futures_util::StreamExt;
    use serde::Serialize;
    use sha2::{Digest, Sha256};
    use sqlx::SqlitePool;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use tokio::io::AsyncWriteExt;
    use tokio::sync::OnceCell;
    use tokio_util::sync::CancellationToken;
    use whisper_rs::WhisperContext;

    const MODEL_FILE: &str = "ggml-base-q5_1.bin";
    const MODEL_URL: &str =
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_1.bin";
    const MODEL_SHA256: &str = "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898";
    pub const MODEL_BYTES: u64 = 59_707_625;
    const MIN_SPEECH_SAMPLES: usize = RECOGNITION_RATE as usize / 4;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub struct Playback {
        pub room_id: String,
        pub message_id: String,
        pub agent_id: Option<String>,
        pub speaking: bool,
    }

    pub type EmitPlayback = Arc<dyn Fn(Playback) + Send + Sync>;

    pub struct Voice {
        model_path: PathBuf,
        recognizer: tokio::sync::Mutex<Option<Arc<WhisperContext>>>,
        download: tokio::sync::Mutex<()>,
        capture: Mutex<Option<Capture>>,
        queue: tokio::sync::Mutex<()>,
        stop: Mutex<CancellationToken>,
        voices: OnceCell<Vec<SystemVoice>>,
        emit: EmitPlayback,
    }

    impl Voice {
        pub fn new(models_dir: &Path, emit: EmitPlayback) -> Voice {
            Voice {
                model_path: models_dir.join(MODEL_FILE),
                recognizer: tokio::sync::Mutex::new(None),
                download: tokio::sync::Mutex::new(()),
                capture: Mutex::new(None),
                queue: tokio::sync::Mutex::new(()),
                stop: Mutex::new(CancellationToken::new()),
                voices: OnceCell::new(),
                emit,
            }
        }

        pub fn status(&self) -> VoiceStatus {
            VoiceStatus {
                available: true,
                model_ready: self.model_path.is_file(),
                model_bytes: MODEL_BYTES,
            }
        }

        pub async fn download_model(
            &self,
            on_progress: impl Fn(u64),
        ) -> Result<VoiceStatus, SessionError> {
            let _guard = self.download.lock().await;
            if self.model_path.is_file() {
                return Ok(self.status());
            }
            let partial = self.model_path.with_extension("part");
            let result = fetch(&self.model_path, &partial, &on_progress).await;
            if result.is_err() {
                let _ = tokio::fs::remove_file(&partial).await;
            }
            result?;
            Ok(self.status())
        }

        pub fn start_capture(&self) -> Result<(), SessionError> {
            if !self.model_path.is_file() {
                return Err(SessionError::VoiceModelMissing);
            }
            self.stop_speaking();
            let mut slot = self
                .capture
                .lock()
                .map_err(|_| SessionError::MicrophoneFailed)?;
            if slot.is_none() {
                *slot = Some(Capture::start().map_err(capture_error)?);
            }
            Ok(())
        }

        pub fn cancel_capture(&self) {
            let taken = self.capture.lock().ok().and_then(|mut slot| slot.take());
            if let Some(capture) = taken {
                let _ = capture.finish();
            }
        }

        pub async fn finish_capture(&self, pool: &SqlitePool) -> Result<String, SessionError> {
            let capture = self
                .capture
                .lock()
                .map_err(|_| SessionError::MicrophoneFailed)?
                .take()
                .ok_or(SessionError::NotCapturing)?;
            let samples = tokio::task::spawn_blocking(move || capture.finish())
                .await
                .map_err(|_| SessionError::MicrophoneFailed)?
                .map_err(capture_error)?;
            if samples.len() < MIN_SPEECH_SAMPLES {
                return Err(SessionError::NothingHeard);
            }
            match speech::heard(&samples) {
                Heard::Muted => return Err(SessionError::MicrophoneUnavailable),
                Heard::Silence => return Err(SessionError::NothingHeard),
                Heard::Sound => {}
            }
            let locale = session_load::require(pool).await?.locale;
            let recognizer = self.recognizer().await?;
            let text = tokio::task::spawn_blocking(move || {
                speech_recognition::transcribe(&recognizer, &samples, locale)
            })
            .await
            .map_err(|_| SessionError::TranscriptionFailed)?
            .map_err(|_| SessionError::TranscriptionFailed)?;
            let text = speech::spoken_text(&text);
            if text.chars().all(|ch| !ch.is_alphanumeric()) {
                return Err(SessionError::NothingHeard);
            }
            Ok(text)
        }

        async fn recognizer(&self) -> Result<Arc<WhisperContext>, SessionError> {
            let mut slot = self.recognizer.lock().await;
            if let Some(loaded) = slot.as_ref() {
                return Ok(loaded.clone());
            }
            let path = self.model_path.clone();
            let loaded = tokio::task::spawn_blocking(move || speech_recognition::load(&path))
                .await
                .map_err(|_| SessionError::VoiceModelUnavailable)?
                .map_err(|_| SessionError::VoiceModelUnavailable)?;
            let loaded = Arc::new(loaded);
            *slot = Some(loaded.clone());
            Ok(loaded)
        }

        pub async fn speak(
            self: Arc<Self>,
            pool: &SqlitePool,
            room_id: &str,
            message_id: &str,
        ) -> Result<(), SessionError> {
            let message = messages::get(pool, room_id, message_id)
                .await?
                .ok_or(SessionError::MessageNotFound)?;
            let Some(agent_id) = message.agent_id.clone() else {
                return Ok(());
            };
            let text = speech::speakable(&message.body);
            if text.is_empty() {
                return Ok(());
            }
            let locale = session_load::require(pool).await?.locale;
            let voice = self.voice(&agent_id, locale).await;
            let stop = self
                .stop
                .lock()
                .map(|token| token.clone())
                .unwrap_or_default();
            let playback = Playback {
                room_id: message.room_id,
                message_id: message.id,
                agent_id: Some(agent_id),
                speaking: true,
            };
            tokio::spawn(async move {
                let _turn = self.queue.lock().await;
                if stop.is_cancelled() {
                    return;
                }
                let Ok(mut child) = speech_synthesis::speak(voice.as_deref(), &text).await else {
                    return;
                };
                (self.emit)(playback.clone());
                tokio::select! {
                    _ = child.wait() => {}
                    _ = stop.cancelled() => { let _ = child.kill().await; }
                }
                (self.emit)(Playback {
                    speaking: false,
                    ..playback
                });
            });
            Ok(())
        }

        pub fn stop_speaking(&self) {
            if let Ok(mut stop) = self.stop.lock() {
                stop.cancel();
                *stop = CancellationToken::new();
            }
        }

        async fn voice(&self, agent_id: &str, locale: Locale) -> Option<String> {
            let voices = self
                .voices
                .get_or_init(|| async {
                    speech_synthesis::voice_listing()
                        .await
                        .map(|listing| speech::parse_voices(&listing))
                        .unwrap_or_default()
                })
                .await;
            speech::voice_for(agent_id, locale, voices).map(|voice| voice.name.clone())
        }
    }

    fn capture_error(code: &'static str) -> SessionError {
        match code {
            "microphone_unavailable" => SessionError::MicrophoneUnavailable,
            _ => SessionError::MicrophoneFailed,
        }
    }

    async fn fetch(
        target: &Path,
        partial: &Path,
        on_progress: &impl Fn(u64),
    ) -> Result<(), SessionError> {
        let failed = |_| SessionError::VoiceModelDownloadFailed;
        if let Some(folder) = target.parent() {
            tokio::fs::create_dir_all(folder).await.map_err(failed)?;
        }
        let response = reqwest::get(MODEL_URL)
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|_| SessionError::VoiceModelDownloadFailed)?;
        let mut file = tokio::fs::File::create(partial).await.map_err(failed)?;
        let mut hasher = Sha256::new();
        let mut written: u64 = 0;
        let mut reported_percent = 0;
        let mut body = response.bytes_stream();
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(|_| SessionError::VoiceModelDownloadFailed)?;
            written += chunk.len() as u64;
            if written > MODEL_BYTES {
                return Err(SessionError::VoiceModelDownloadFailed);
            }
            hasher.update(&chunk);
            file.write_all(&chunk).await.map_err(failed)?;
            let percent = written * 100 / MODEL_BYTES;
            if percent != reported_percent {
                reported_percent = percent;
                on_progress(written);
            }
        }
        file.sync_all().await.map_err(failed)?;
        drop(file);
        let digest: String = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        if written != MODEL_BYTES || digest != MODEL_SHA256 {
            return Err(SessionError::VoiceModelDownloadFailed);
        }
        tokio::fs::rename(partial, target).await.map_err(failed)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn voice(dir: &Path) -> Voice {
            Voice::new(dir, Arc::new(|_| {}))
        }

        #[test]
        fn reports_the_model_missing_until_it_is_on_disk() {
            let dir = tempfile::tempdir().unwrap();
            let voice = voice(dir.path());

            assert_eq!(
                voice.status(),
                VoiceStatus {
                    available: true,
                    model_ready: false,
                    model_bytes: MODEL_BYTES
                }
            );
            assert!(matches!(
                voice.start_capture(),
                Err(SessionError::VoiceModelMissing)
            ));

            std::fs::write(dir.path().join(MODEL_FILE), b"model").unwrap();
            assert!(voice.status().model_ready);
        }

        #[tokio::test]
        async fn finishing_without_a_capture_is_refused() {
            let dir = tempfile::tempdir().unwrap();
            let (pool, _db) = crate::repositories::database::open_for_test().await;

            assert!(matches!(
                voice(dir.path()).finish_capture(&pool).await,
                Err(SessionError::NotCapturing)
            ));
        }

        #[tokio::test]
        #[ignore = "downloads the 57 MiB speech model from Hugging Face; set MEELTING_VOICE_MODEL_DIR to keep it"]
        async fn downloads_and_verifies_the_model_then_transcribes_spoken_spanish() {
            let dir = tempfile::tempdir().unwrap();
            let models = std::env::var("MEELTING_VOICE_MODEL_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| dir.path().to_path_buf());
            let voice = voice(&models);

            assert!(voice.download_model(|_| {}).await.unwrap().model_ready);

            let wav = dir.path().join("sample.wav");
            let status = std::process::Command::new("/usr/bin/say")
                .args(["-o", wav.to_str().unwrap(), "--data-format=LEI16@16000"])
                .arg("Hola Ada, cambia la primera línea del archivo.")
                .status()
                .unwrap();
            assert!(status.success());
            let bytes = std::fs::read(&wav).unwrap();
            let start = bytes.windows(4).position(|w| w == b"data").unwrap() + 8;
            let samples: Vec<f32> = bytes[start..]
                .chunks_exact(2)
                .map(|pair| f32::from(i16::from_le_bytes([pair[0], pair[1]])) / 32_768.0)
                .collect();
            let recognizer = voice.recognizer().await.unwrap();
            let text = speech_recognition::transcribe(&recognizer, &samples, Locale::Es).unwrap();

            println!("{text}");
            assert!(text.to_lowercase().contains("primera línea"), "{text}");
        }
    }
}
