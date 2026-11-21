#[derive(Debug, PartialEq, Eq)]
pub struct SpeechModel {
    pub name: &'static str,
    pub file: &'static str,
    pub sha256: &'static str,
    pub bytes: u64,
    pub working_bytes: u64,
}

pub const LARGE_V3_TURBO: SpeechModel = SpeechModel {
    name: "Whisper large-v3 turbo",
    file: "ggml-large-v3-turbo-q5_0.bin",
    sha256: "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
    bytes: 574_041_195,
    working_bytes: 1_500_000_000,
};

pub const SMALL: SpeechModel = SpeechModel {
    name: "Whisper small",
    file: "ggml-small-q5_1.bin",
    sha256: "ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb",
    bytes: 190_085_487,
    working_bytes: 700_000_000,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    pub chip: String,
    pub memory_bytes: u64,
    pub apple_silicon: bool,
}

pub fn recommended(machine: &Machine) -> &'static SpeechModel {
    if machine.apple_silicon && fits_comfortably(&LARGE_V3_TURBO, machine.memory_bytes) {
        &LARGE_V3_TURBO
    } else {
        &SMALL
    }
}

pub fn download_url(model: &SpeechModel) -> String {
    format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
        model.file
    )
}

fn fits_comfortably(model: &SpeechModel, memory_bytes: u64) -> bool {
    u128::from(model.working_bytes) * 40 <= u128::from(memory_bytes) * 21
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;

    fn machine(apple_silicon: bool, memory_bytes: u64) -> Machine {
        Machine {
            chip: "chip".into(),
            memory_bytes,
            apple_silicon,
        }
    }

    #[test]
    fn recommends_large_v3_turbo_on_apple_silicon_with_room_for_it() {
        assert_eq!(recommended(&machine(true, 8 * GIB)), &LARGE_V3_TURBO);
        assert_eq!(recommended(&machine(true, 32 * GIB)), &LARGE_V3_TURBO);
    }

    #[test]
    fn recommends_small_without_metal_or_without_comfortable_memory() {
        assert_eq!(recommended(&machine(false, 64 * GIB)), &SMALL);
        assert_eq!(recommended(&machine(true, 2 * GIB)), &SMALL);
    }

    #[test]
    fn keeps_almost_half_of_the_memory_free_for_everything_else() {
        assert!(fits_comfortably(&LARGE_V3_TURBO, 2_858_000_000));
        assert!(!fits_comfortably(&LARGE_V3_TURBO, 2_857_000_000));
    }

    #[test]
    fn downloads_from_the_whisper_cpp_repository() {
        assert_eq!(
            download_url(&SMALL),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small-q5_1.bin"
        );
    }
}
