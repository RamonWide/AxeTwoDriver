//! USB descriptor parsing for AxeTwoDriver. Pure data in, data out: no system
//! calls, no `unsafe`, and no panics on malformed input.
//!
//! Field layouts follow the USB 2.0 specification (chapter 9) and the USB
//! Device Class Definition for Audio Devices, release 2.0.

pub mod layout;

use std::fmt;

/// Why a descriptor could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The data ended inside a descriptor.
    Truncated,
    /// A descriptor declared a length shorter than its own header.
    BadLength,
    /// The first descriptor is not a configuration descriptor.
    NotAConfiguration,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Truncated => "the descriptor data ends inside a descriptor",
            Self::BadLength => "a descriptor declares an impossible length",
            Self::NotAConfiguration => "the data does not start with a configuration descriptor",
        })
    }
}

impl std::error::Error for ParseError {}

pub const CLASS_AUDIO: u8 = 0x01;
pub const SUBCLASS_AUDIO_CONTROL: u8 = 0x01;
pub const SUBCLASS_AUDIO_STREAMING: u8 = 0x02;
pub const SUBCLASS_MIDI_STREAMING: u8 = 0x03;
/// `bInterfaceProtocol` of an Audio Class 2.0 interface.
pub const PROTOCOL_AUDIO_2: u8 = 0x20;

const DT_CONFIGURATION: u8 = 0x02;
const DT_INTERFACE: u8 = 0x04;
const DT_ENDPOINT: u8 = 0x05;
const DT_CS_INTERFACE: u8 = 0x24;

/// How an isochronous endpoint keeps time (bits 3..2 of `bmAttributes`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sync {
    None,
    Asynchronous,
    Adaptive,
    Synchronous,
}

/// What an isochronous endpoint carries (bits 5..4 of `bmAttributes`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Usage {
    Data,
    Feedback,
    /// A data endpoint whose rate also serves as feedback for the other direction.
    ImplicitFeedback,
    Reserved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Endpoint {
    /// Endpoint address; bit 7 set means device-to-host.
    pub address: u8,
    pub attributes: u8,
    pub max_packet: u16,
    pub interval: u8,
}

impl Endpoint {
    pub fn is_in(&self) -> bool {
        self.address & 0x80 != 0
    }

    pub fn is_isochronous(&self) -> bool {
        self.attributes & 0x03 == 1
    }

    pub fn sync(&self) -> Sync {
        match (self.attributes >> 2) & 0x03 {
            0 => Sync::None,
            1 => Sync::Asynchronous,
            2 => Sync::Adaptive,
            _ => Sync::Synchronous,
        }
    }

    pub fn usage(&self) -> Usage {
        match (self.attributes >> 4) & 0x03 {
            0 => Usage::Data,
            1 => Usage::Feedback,
            2 => Usage::ImplicitFeedback,
            _ => Usage::Reserved,
        }
    }
}

/// One alternate setting of an interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    pub number: u8,
    pub alternate: u8,
    pub class: u8,
    pub subclass: u8,
    pub protocol: u8,
    pub endpoints: Vec<Endpoint>,
    /// Class-specific interface descriptors, each complete with its header.
    pub class_specific: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Configuration {
    pub value: u8,
    pub interfaces: Vec<Interface>,
}

/// Parses a complete configuration descriptor.
pub fn parse_configuration(data: &[u8]) -> Result<Configuration, ParseError> {
    let mut config: Option<Configuration> = None;
    let mut rest = data;
    while !rest.is_empty() {
        let len = usize::from(*rest.first().ok_or(ParseError::Truncated)?);
        if len < 2 {
            return Err(ParseError::BadLength);
        }
        let d = rest.get(..len).ok_or(ParseError::Truncated)?;
        rest = rest.get(len..).ok_or(ParseError::Truncated)?;
        let kind = *d.get(1).ok_or(ParseError::Truncated)?;
        match (&mut config, kind) {
            (None, DT_CONFIGURATION) => {
                let value = *d.get(5).ok_or(ParseError::BadLength)?;
                config = Some(Configuration { value, interfaces: Vec::new() });
            }
            (None, _) => return Err(ParseError::NotAConfiguration),
            (Some(c), DT_INTERFACE) => {
                let f = d.get(..9).ok_or(ParseError::BadLength)?;
                c.interfaces.push(Interface {
                    number: f[2],
                    alternate: f[3],
                    class: f[5],
                    subclass: f[6],
                    protocol: f[7],
                    endpoints: Vec::new(),
                    class_specific: Vec::new(),
                });
            }
            (Some(c), DT_ENDPOINT) => {
                let f = d.get(..7).ok_or(ParseError::BadLength)?;
                if let Some(i) = c.interfaces.last_mut() {
                    i.endpoints.push(Endpoint {
                        address: f[2],
                        attributes: f[3],
                        max_packet: u16::from_le_bytes([f[4], f[5]]),
                        interval: f[6],
                    });
                }
            }
            (Some(c), DT_CS_INTERFACE) => {
                if let Some(i) = c.interfaces.last_mut() {
                    i.class_specific.push(d.to_vec());
                }
            }
            // Association, class-specific endpoint and unknown descriptors are skipped.
            (Some(_), _) => {}
        }
    }
    config.ok_or(ParseError::NotAConfiguration)
}

/// An Audio Class 2.0 streaming alternate setting that carries PCM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioStream {
    pub interface: u8,
    pub alternate: u8,
    pub endpoint: Endpoint,
    pub channels: u8,
    /// Bytes each sample occupies on the wire.
    pub subslot_bytes: u8,
    /// Significant bits per sample.
    pub bits: u8,
}

impl AudioStream {
    /// Bytes of one frame (one sample for every channel).
    pub fn frame_bytes(&self) -> usize {
        usize::from(self.channels) * usize::from(self.subslot_bytes)
    }
}

/// Every Audio Class 2.0 PCM stream in `config`.
pub fn audio_streams(config: &Configuration) -> Vec<AudioStream> {
    let mut found = Vec::new();
    for i in &config.interfaces {
        if i.class != CLASS_AUDIO || i.subclass != SUBCLASS_AUDIO_STREAMING || i.protocol != PROTOCOL_AUDIO_2 {
            continue;
        }
        let Some(endpoint) = i.endpoints.iter().find(|e| e.is_isochronous()).copied() else {
            continue;
        };
        // AS_GENERAL (subtype 1): bFormatType at 5, bmFormats at 6..10, bNrChannels at 10.
        let general = i.class_specific.iter().find(|d| d.get(2) == Some(&0x01) && d.len() >= 11);
        // FORMAT_TYPE (subtype 2), type I: bSubslotSize at 4, bBitResolution at 5.
        let format =
            i.class_specific.iter().find(|d| d.get(2) == Some(&0x02) && d.get(3) == Some(&0x01) && d.len() >= 6);
        let (Some(general), Some(format)) = (general, format) else {
            continue;
        };
        let pcm = general.get(6).is_some_and(|b| b & 0x01 != 0);
        if !pcm {
            continue;
        }
        found.push(AudioStream {
            interface: i.number,
            alternate: i.alternate,
            endpoint,
            channels: general.get(10).copied().unwrap_or(0),
            subslot_bytes: format.get(4).copied().unwrap_or(0),
            bits: format.get(5).copied().unwrap_or(0),
        });
    }
    found
}

/// IDs of the clock sources declared by Audio Class 2.0 control interfaces.
pub fn clock_sources(config: &Configuration) -> Vec<u8> {
    config
        .interfaces
        .iter()
        .filter(|i| i.class == CLASS_AUDIO && i.subclass == SUBCLASS_AUDIO_CONTROL && i.protocol == PROTOCOL_AUDIO_2)
        .flat_map(|i| i.class_specific.iter())
        // CLOCK_SOURCE is subtype 0x0A; bClockID at 3.
        .filter(|d| d.get(2) == Some(&0x0a))
        .filter_map(|d| d.get(3).copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_garbage_without_panicking() {
        assert_eq!(parse_configuration(&[]), Err(ParseError::NotAConfiguration));
        assert_eq!(parse_configuration(&[0]), Err(ParseError::BadLength));
        assert_eq!(parse_configuration(&[9, 2, 0]), Err(ParseError::Truncated));
        assert_eq!(parse_configuration(&[2, 4]), Err(ParseError::NotAConfiguration));
        // A configuration header too short to hold its value.
        assert_eq!(parse_configuration(&[3, 2, 0]), Err(ParseError::BadLength));
    }

    #[test]
    fn every_prefix_of_a_real_descriptor_is_handled() {
        let full = layout::tests::CONFIGURATION;
        for n in 0..full.len() {
            let _ = parse_configuration(&full[..n]);
        }
    }

    #[test]
    fn endpoint_attributes_decode() {
        let e = Endpoint { address: 0x86, attributes: 0x25, max_packet: 84, interval: 1 };
        assert!(e.is_in());
        assert!(e.is_isochronous());
        assert_eq!(e.sync(), Sync::Asynchronous);
        assert_eq!(e.usage(), Usage::ImplicitFeedback);
    }
}
