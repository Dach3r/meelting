use crate::domain::locale::Locale;
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub fn load(model: &Path) -> Result<WhisperContext, &'static str> {
    let path = model.to_str().ok_or("voice_model_unavailable")?;
    whisper_rs::install_logging_hooks();
    WhisperContext::new_with_params(path, WhisperContextParameters::default())
        .map_err(|_| "voice_model_unavailable")
}

pub fn transcribe(
    context: &WhisperContext,
    samples: &[f32],
    locale: Locale,
) -> Result<String, &'static str> {
    let mut state = context.create_state().map_err(|_| "transcription_failed")?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some(locale.as_str()));
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_print_special(false);
    params.set_suppress_blank(true);
    state
        .full(params, samples)
        .map_err(|_| "transcription_failed")?;
    let text: String = state.as_iter().map(|segment| segment.to_string()).collect();
    Ok(text.trim().to_string())
}
