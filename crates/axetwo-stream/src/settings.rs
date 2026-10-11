//! The user's streaming settings, shared by the control panel and by
//! whatever front end streams audio.

/// USB streaming modes: milliseconds of playback kept queued ahead of the
/// unit, and the name each goes by. More is safer against dropouts and adds
/// latency.
pub const STREAMING_MODES: [(u32, &str); 7] = [
    (1, "Minimum Latency"),
    (2, "Low Latency"),
    (4, "Standard"),
    (8, "Relaxed"),
    (12, "Reliable"),
    (16, "Safe"),
    (32, "Extra Safe"),
];

/// Buffer sizes offered to audio programs, in frames.
pub const BUFFER_SIZES: [u32; 7] = [64, 128, 256, 512, 1024, 2048, 4096];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// One of the values in [`STREAMING_MODES`].
    pub streaming_ms: u32,
    /// One of [`BUFFER_SIZES`].
    pub buffer_frames: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self { streaming_ms: 4, buffer_frames: 256 }
    }
}

impl Settings {
    /// Replaces values that are not on the lists with the defaults.
    pub fn sanitized(self) -> Self {
        let d = Self::default();
        Self {
            streaming_ms: if STREAMING_MODES.iter().any(|m| m.0 == self.streaming_ms) {
                self.streaming_ms
            } else {
                d.streaming_ms
            },
            buffer_frames: if BUFFER_SIZES.contains(&self.buffer_frames) {
                self.buffer_frames
            } else {
                d.buffer_frames
            },
        }
    }

    /// Whether the buffer lasts less time than the streaming mode queues.
    /// The original driver warns about this combination; it is kept as a hint,
    /// not enforced.
    pub fn buffer_too_small(&self, sample_rate: u32) -> bool {
        u64::from(self.buffer_frames) * 1000 < u64::from(self.streaming_ms) * u64::from(sample_rate)
    }
}

#[cfg(windows)]
mod store {
    use super::Settings;
    use axetwo_win::registry::{self, Root};
    use axetwo_win::WinError;

    const KEY: &str = "Software\\AxeTwoDriver";
    const STREAMING: &str = "StreamingModeMs";
    const BUFFER: &str = "BufferSize";

    impl Settings {
        /// The saved settings, with defaults for anything missing or invalid.
        pub fn load() -> Self {
            let d = Self::default();
            Self {
                streaming_ms: registry::get_dword(Root::CurrentUser, KEY, STREAMING).unwrap_or(d.streaming_ms),
                buffer_frames: registry::get_dword(Root::CurrentUser, KEY, BUFFER).unwrap_or(d.buffer_frames),
            }
            .sanitized()
        }

        pub fn save(&self) -> Result<(), WinError> {
            let s = self.sanitized();
            registry::set_dword(Root::CurrentUser, KEY, STREAMING, s.streaming_ms)?;
            registry::set_dword(Root::CurrentUser, KEY, BUFFER, s.buffer_frames)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_values_fall_back_to_defaults() {
        let s = Settings { streaming_ms: 3, buffer_frames: 100 }.sanitized();
        assert_eq!(s, Settings::default());
        let ok = Settings { streaming_ms: 32, buffer_frames: 4096 };
        assert_eq!(ok.sanitized(), ok);
    }

    #[test]
    fn the_default_is_on_both_lists() {
        assert_eq!(Settings::default().sanitized(), Settings::default());
    }

    #[test]
    fn small_buffers_are_flagged_against_long_modes() {
        // 64 frames is 1.33 ms at 48 kHz.
        assert!(Settings { streaming_ms: 2, buffer_frames: 64 }.buffer_too_small(48_000));
        assert!(!Settings { streaming_ms: 1, buffer_frames: 64 }.buffer_too_small(48_000));
        assert!(!Settings { streaming_ms: 4, buffer_frames: 256 }.buffer_too_small(48_000));
    }
}
