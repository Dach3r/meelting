use crate::domain::locale::Locale;
use rubato::audioadapter_buffers::owned::InterleavedOwned;
use rubato::{Fft, FixedSync, Resampler};

pub const RECOGNITION_RATE: u32 = 16_000;
pub const MAX_SPOKEN_CHARS: usize = 1_200;

const NOVELTY_VOICES: [&str; 19] = [
    "Albert",
    "Bad News",
    "Bahh",
    "Bells",
    "Boing",
    "Bubbles",
    "Cellos",
    "Fred",
    "Good News",
    "Jester",
    "Junior",
    "Kathy",
    "Organ",
    "Ralph",
    "Superstar",
    "Trinoids",
    "Whisper",
    "Wobble",
    "Zarvox",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemVoice {
    pub name: String,
    pub language: String,
}

pub fn parse_voices(listing: &str) -> Vec<SystemVoice> {
    listing
        .lines()
        .filter_map(|line| {
            let (head, _) = line.split_once('#')?;
            let (name, language) = head.trim_end().rsplit_once(char::is_whitespace)?;
            let name = name.trim();
            let language = language.trim();
            let shaped = language.len() >= 5 && language.as_bytes()[2] == b'_';
            (shaped && !name.is_empty()).then(|| SystemVoice {
                name: name.to_string(),
                language: language.to_string(),
            })
        })
        .collect()
}

pub fn voice_for<'a>(
    agent_id: &str,
    locale: Locale,
    voices: &'a [SystemVoice],
) -> Option<&'a SystemVoice> {
    let prefix = format!("{}_", locale.as_str());
    let mut candidates: Vec<&SystemVoice> = voices
        .iter()
        .filter(|voice| voice.language.starts_with(&prefix))
        .filter(|voice| !NOVELTY_VOICES.contains(&voice.name.as_str()))
        .collect();
    if candidates.is_empty() {
        return None;
    }
    candidates.sort_by(|a, b| a.name.cmp(&b.name));
    let hash = agent_id.bytes().fold(2_166_136_261_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16_777_619)
    });
    Some(candidates[hash as usize % candidates.len()])
}

pub fn speakable(text: &str) -> String {
    let mut spoken = String::new();
    let mut in_code = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        let line = line
            .trim_start()
            .trim_start_matches(['#', '>', '-', '*', '+'])
            .trim();
        if line.is_empty() {
            continue;
        }
        let mut cleaned = String::with_capacity(line.len());
        let mut chars = line.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '*' | '_' | '`' | '~' => {}
                '[' => {
                    let label: String = chars.by_ref().take_while(|&c| c != ']').collect();
                    cleaned.push_str(&label);
                    if chars.peek() == Some(&'(') {
                        chars.by_ref().take_while(|&c| c != ')').for_each(drop);
                    }
                }
                _ => cleaned.push(ch),
            }
        }
        if !spoken.is_empty() {
            spoken.push(if spoken.ends_with(['.', '!', '?', ':']) {
                ' '
            } else {
                '.'
            });
            if !spoken.ends_with(' ') {
                spoken.push(' ');
            }
        }
        spoken.push_str(cleaned.trim());
    }
    shorten(&spoken)
}

fn shorten(text: &str) -> String {
    if text.chars().count() <= MAX_SPOKEN_CHARS {
        return text.to_string();
    }
    let cut: String = text.chars().take(MAX_SPOKEN_CHARS).collect();
    match cut.rfind(['.', '!', '?']) {
        Some(end) if end > MAX_SPOKEN_CHARS / 2 => cut[..=end].to_string(),
        _ => cut,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Heard {
    Muted,
    Silence,
    Sound,
}

const SILENCE_PEAK: f32 = 0.003;
const TARGET_PEAK: f32 = 0.9;
const MAX_GAIN: f32 = 10.0;
const RESAMPLE_CHUNK_FRAMES: usize = 1024;

pub fn heard(samples: &[f32]) -> Heard {
    let peak = samples
        .iter()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    if peak == 0.0 {
        Heard::Muted
    } else if peak < SILENCE_PEAK {
        Heard::Silence
    } else {
        Heard::Sound
    }
}

pub fn spoken_text(transcript: &str) -> String {
    let mut text = String::new();
    let mut depth = 0_u32;
    for ch in transcript.chars() {
        match ch {
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => text.push(ch),
            _ => {}
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn to_recognition_rate(
    samples: &[f32],
    channels: u16,
    rate: u32,
) -> Result<Vec<f32>, &'static str> {
    let channels = usize::from(channels.max(1));
    let mono: Vec<f32> = samples
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect();
    if rate == RECOGNITION_RATE || mono.is_empty() {
        return Ok(mono);
    }
    let frames = mono.len();
    let input = InterleavedOwned::new_from(mono, 1, frames).map_err(|_| "microphone_failed")?;
    let mut resampler = Fft::<f32>::new(
        rate as usize,
        RECOGNITION_RATE as usize,
        RESAMPLE_CHUNK_FRAMES,
        1,
        FixedSync::Both,
    )
    .map_err(|_| "microphone_failed")?;
    let output = resampler
        .process_all(&input, frames, None)
        .map_err(|_| "microphone_failed")?;
    Ok(output.take_data())
}

pub fn normalize_level(samples: &mut [f32]) {
    let peak = samples
        .iter()
        .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
    if peak < SILENCE_PEAK {
        return;
    }
    let gain = (TARGET_PEAK / peak).min(MAX_GAIN);
    for sample in samples {
        *sample *= gain;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING: &str = "Albert              en_US    # Hello! My name is Albert.\n\
Eddy (Español (España)) es_ES    # ¡Hola! Me llamo Eddy.\n\
Mónica              es_ES    # ¡Hola! Me llamo Mónica.\n\
Paulina             es_MX    # ¡Hola! Me llamo Paulina.\n\
Samantha            en_US    # Hello! My name is Samantha.\n\
broken line without a hash\n";

    #[test]
    fn parses_the_system_voice_listing() {
        let voices = parse_voices(LISTING);

        assert_eq!(voices.len(), 5);
        assert_eq!(
            voices[1],
            SystemVoice {
                name: "Eddy (Español (España))".into(),
                language: "es_ES".into()
            }
        );
    }

    #[test]
    fn each_agent_keeps_a_stable_voice_of_the_language_and_never_a_novelty_one() {
        let voices = parse_voices(LISTING);

        let first = voice_for("agent-1", Locale::Es, &voices).unwrap();
        assert!(first.language.starts_with("es_"));
        assert_eq!(voice_for("agent-1", Locale::Es, &voices), Some(first));
        for id in ["a", "b", "c", "d", "e", "f"] {
            assert_eq!(voice_for(id, Locale::En, &voices).unwrap().name, "Samantha");
        }
        assert_eq!(voice_for("a", Locale::En, &voices[..1]), None);
    }

    #[test]
    fn speaks_prose_without_code_or_markdown() {
        let text = "## Done\nI changed **README.md**:\n```diff\n-hello\n+Hello\n```\n- See [the docs](https://x.y) and `main.rs`";

        assert_eq!(
            speakable(text),
            "Done. I changed README.md: See the docs and main.rs"
        );
    }

    #[test]
    fn long_answers_end_at_a_sentence() {
        let text = "One sentence. ".repeat(200);

        let spoken = speakable(&text);

        assert!(spoken.chars().count() <= MAX_SPOKEN_CHARS);
        assert!(spoken.ends_with('.'));
    }

    #[test]
    fn tells_a_muted_microphone_from_silence_and_speech() {
        assert_eq!(heard(&[0.0; 8_000]), Heard::Muted);
        assert_eq!(heard(&[0.001, -0.002, 0.0]), Heard::Silence);
        assert_eq!(heard(&[0.001, -0.2, 0.0]), Heard::Sound);
    }

    #[test]
    fn drops_the_recognizer_annotations_from_the_transcript() {
        assert_eq!(spoken_text("[BLANK_AUDIO]"), "");
        assert_eq!(
            spoken_text(" (música) Ana,  cambia el título [risas]"),
            "Ana, cambia el título"
        );
        assert_eq!(spoken_text("Hola Ada."), "Hola Ada.");
    }

    fn sine(rate: u32, hertz: f32, seconds: u32) -> Vec<f32> {
        (0..rate * seconds)
            .map(|index| (std::f32::consts::TAU * hertz * index as f32 / rate as f32).sin() * 0.5)
            .collect()
    }

    fn rms(samples: &[f32]) -> f32 {
        let middle = &samples[samples.len() / 4..samples.len() * 3 / 4];
        (middle.iter().map(|sample| sample * sample).sum::<f32>() / middle.len() as f32).sqrt()
    }

    #[test]
    fn mixes_channels_and_resamples_to_sixteen_kilohertz() {
        let stereo: Vec<f32> = (0..96_000).flat_map(|_| [0.5, -0.5]).collect();

        let mono = to_recognition_rate(&stereo, 2, 48_000).unwrap();

        assert!((mono.len() as i64 - 32_000).abs() <= 2);
        assert!(mono.iter().all(|sample| sample.abs() < 1e-6));
        assert_eq!(
            to_recognition_rate(&[0.25; 10], 1, 16_000).unwrap(),
            vec![0.25; 10]
        );
    }

    #[test]
    fn keeps_speech_frequencies_when_downsampling() {
        let tone = to_recognition_rate(&sine(48_000, 1_000.0, 2), 1, 48_000).unwrap();

        assert!((rms(&tone) - 0.5 / 2.0_f32.sqrt()).abs() < 0.02);
    }

    #[test]
    fn removes_frequencies_above_the_recognition_band_instead_of_folding_them_back() {
        let ultrasound = to_recognition_rate(&sine(48_000, 20_000.0, 2), 1, 48_000).unwrap();

        assert!(rms(&ultrasound) < 0.02);
    }

    #[test]
    fn raises_a_quiet_voice_without_clipping_or_amplifying_silence() {
        let mut quiet = sine(16_000, 300.0, 1)
            .iter()
            .map(|s| s * 0.05)
            .collect::<Vec<_>>();
        normalize_level(&mut quiet);
        let peak = quiet.iter().fold(0.0_f32, |peak, s| peak.max(s.abs()));
        assert!((peak - 0.25).abs() < 0.01);

        let mut faint = vec![0.0, 0.001, -0.002];
        normalize_level(&mut faint);
        assert_eq!(faint, vec![0.0, 0.001, -0.002]);

        let mut loud = vec![0.95, -0.95];
        normalize_level(&mut loud);
        assert!(loud.iter().all(|s| s.abs() <= 0.9 + 1e-6));
    }
}
