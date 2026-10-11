//! The stream layout AxeTwoDriver knows how to drive, checked against what a
//! device actually reports before any audio is exchanged.

use crate::{audio_streams, clock_sources, AudioStream, Configuration, Sync, Usage};
use std::fmt;

/// USB vendor id of Fractal Audio Systems.
pub const VENDOR_ID: u16 = 0x2466;
/// Product id of an Axe-Fx II whose USB firmware is running.
pub const PRODUCT_ID_RUNNING: u16 = 0x8003;
/// Product id of an Axe-Fx II waiting for its USB firmware.
pub const PRODUCT_ID_BOOTLOADER: u16 = 0x0003;

/// The audio streams of a unit, as needed to exchange audio with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamLayout {
    /// Host-to-device stream (what the computer plays).
    pub playback: AudioStream,
    /// Device-to-host stream (what the computer records).
    pub capture: AudioStream,
    pub clock_id: u8,
}

/// Why a device's descriptors are not a layout this driver supports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutError {
    NoPlayback,
    NoCapture,
    NoClock,
    /// The capture endpoint cannot serve as the rate reference for playback.
    NoRateReference,
    UnsupportedFormat,
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NoPlayback => "the device declares no playback stream",
            Self::NoCapture => "the device declares no capture stream",
            Self::NoClock => "the device declares no clock source",
            Self::NoRateReference => "the capture endpoint is not an asynchronous implicit-feedback endpoint",
            Self::UnsupportedFormat => "the streams are not 24-bit PCM in 3-byte samples",
        })
    }
}

impl std::error::Error for LayoutError {}

/// Derives the stream layout from a parsed configuration.
pub fn stream_layout(config: &Configuration) -> Result<StreamLayout, LayoutError> {
    let streams = audio_streams(config);
    let playback = streams.iter().find(|s| !s.endpoint.is_in()).copied().ok_or(LayoutError::NoPlayback)?;
    let capture = streams.iter().find(|s| s.endpoint.is_in()).copied().ok_or(LayoutError::NoCapture)?;
    let clock_id = clock_sources(config).first().copied().ok_or(LayoutError::NoClock)?;
    if capture.endpoint.sync() != Sync::Asynchronous || capture.endpoint.usage() != Usage::ImplicitFeedback {
        return Err(LayoutError::NoRateReference);
    }
    for s in [&playback, &capture] {
        if s.subslot_bytes != 3 || s.bits != 24 || s.channels == 0 {
            return Err(LayoutError::UnsupportedFormat);
        }
        if usize::from(s.endpoint.max_packet) < s.frame_bytes() {
            return Err(LayoutError::UnsupportedFormat);
        }
    }
    Ok(StreamLayout { playback, capture, clock_id })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::parse_configuration;

    /// Configuration descriptor reported by an Axe-Fx II XL (USB firmware
    /// running, product id 8003), read with `axetwo probe`.
    pub(crate) const CONFIGURATION: &[u8] = &[
        0x09, 0x02, 0x2f, 0x01, 0x05, 0x01, 0x00, 0xc0, 0x01, //
        0x08, 0x0b, 0x00, 0x03, 0x01, 0x00, 0x20, 0x08, //
        0x09, 0x04, 0x00, 0x00, 0x00, 0x01, 0x01, 0x20, 0x03, //
        0x09, 0x24, 0x01, 0x00, 0x02, 0x08, 0x4b, 0x00, 0x00, //
        0x08, 0x24, 0x0a, 0x28, 0x01, 0x01, 0x00, 0x04, //
        0x11, 0x24, 0x02, 0x02, 0x01, 0x01, 0x00, 0x28, 0x02, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, //
        0x0c, 0x24, 0x03, 0x14, 0x01, 0x03, 0x00, 0x02, 0x28, 0x00, 0x00, 0x05, //
        0x11, 0x24, 0x02, 0x01, 0x01, 0x06, 0x00, 0x28, 0x04, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, //
        0x0c, 0x24, 0x03, 0x16, 0x01, 0x01, 0x00, 0x01, 0x28, 0x00, 0x00, 0x06, //
        0x09, 0x04, 0x01, 0x00, 0x00, 0x01, 0x02, 0x20, 0x05, //
        0x09, 0x04, 0x01, 0x01, 0x01, 0x01, 0x02, 0x20, 0x05, //
        0x10, 0x24, 0x01, 0x02, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x02, 0x03, 0x00, 0x00, 0x00, 0x00, //
        0x06, 0x24, 0x02, 0x01, 0x03, 0x18, //
        0x07, 0x05, 0x02, 0x05, 0x2a, 0x00, 0x01, //
        0x08, 0x25, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, //
        0x09, 0x04, 0x02, 0x00, 0x00, 0x01, 0x02, 0x20, 0x06, //
        0x09, 0x04, 0x02, 0x01, 0x01, 0x01, 0x02, 0x20, 0x06, //
        0x10, 0x24, 0x01, 0x16, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x04, 0x0f, 0x00, 0x00, 0x00, 0x00, //
        0x06, 0x24, 0x02, 0x01, 0x03, 0x18, //
        0x07, 0x05, 0x86, 0x25, 0x54, 0x00, 0x01, //
        0x08, 0x25, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, //
        0x09, 0x04, 0x03, 0x00, 0x00, 0x01, 0x01, 0x00, 0x08, //
        0x09, 0x24, 0x01, 0x00, 0x01, 0x09, 0x00, 0x01, 0x04, //
        0x09, 0x04, 0x04, 0x00, 0x02, 0x01, 0x03, 0x00, 0x07, //
        0x07, 0x24, 0x01, 0x00, 0x01, 0x41, 0x00, //
        0x06, 0x24, 0x02, 0x01, 0x01, 0x09, //
        0x06, 0x24, 0x02, 0x02, 0x02, 0x09, //
        0x09, 0x24, 0x03, 0x01, 0x03, 0x01, 0x02, 0x01, 0x0a, //
        0x09, 0x24, 0x03, 0x02, 0x04, 0x01, 0x01, 0x01, 0x0a, //
        0x09, 0x05, 0x04, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, //
        0x05, 0x25, 0x01, 0x01, 0x01, //
        0x09, 0x05, 0x81, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, //
        0x05, 0x25, 0x01, 0x01, 0x03,
    ];

    #[test]
    fn the_captured_descriptor_is_complete() {
        assert_eq!(CONFIGURATION.len(), 303);
    }

    #[test]
    fn axe_fx_ii_xl_layout() {
        let config = parse_configuration(CONFIGURATION).expect("parses");
        assert_eq!(config.value, 1);
        let layout = stream_layout(&config).expect("supported");
        assert_eq!(layout.clock_id, 0x28);
        assert_eq!(layout.playback.interface, 1);
        assert_eq!(layout.playback.alternate, 1);
        assert_eq!(layout.playback.endpoint.address, 0x02);
        assert_eq!(layout.playback.channels, 2);
        assert_eq!(layout.playback.frame_bytes(), 6);
        assert_eq!(layout.playback.endpoint.max_packet, 42);
        assert_eq!(layout.capture.interface, 2);
        assert_eq!(layout.capture.endpoint.address, 0x86);
        assert_eq!(layout.capture.channels, 4);
        assert_eq!(layout.capture.frame_bytes(), 12);
        assert_eq!(layout.capture.endpoint.max_packet, 84);
    }

    #[test]
    fn a_capture_endpoint_without_implicit_feedback_is_refused() {
        let mut bytes = CONFIGURATION.to_vec();
        let at = bytes.windows(4).position(|w| w == [0x07, 0x05, 0x86, 0x25]).expect("capture endpoint present");
        bytes[at + 3] = 0x05;
        let config = parse_configuration(&bytes).expect("parses");
        assert_eq!(stream_layout(&config), Err(LayoutError::NoRateReference));
    }
}
