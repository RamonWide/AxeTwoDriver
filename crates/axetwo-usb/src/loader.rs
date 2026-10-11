//! The firmware-loading script of the original driver package, parsed into
//! the steps it describes.
//!
//! The parser is deliberately narrow: it accepts exactly the one shape of
//! record the known scripts consist of, and only steps that write to the USB
//! controller's RAM or hold and release its reset line. Anything else is
//! refused, so a file this code does not fully understand is never sent to a
//! unit. See `spec/SPEC.md`, section 1.1.

use std::fmt;

/// Vendor request "firmware load", handled by the controller itself.
pub const REQUEST_FIRMWARE_LOAD: u8 = 0xA0;
/// Address of the controller's CPU control register; bit 0 holds it in reset.
pub const CPU_CONTROL: u16 = 0xE600;
/// End of the controller's internal RAM.
pub const RAM_END: usize = 0x4000;

const TAG: &[u8; 4] = b"CSPT";
const HEADER_LEN: usize = 32;
/// Bytes 12..16 of every known record.
const KNOWN_PREFIX: [u8; 4] = [0x00, 0x00, 0x00, 0x50];
/// Byte 17 of every known record.
const KNOWN_FLAGS: u8 = 0x70;
/// The largest data block accepted in one record.
const MAX_BLOCK: usize = 4096;

/// One thing to do to the controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Hold the CPU in reset.
    Hold,
    /// Release the CPU so the loaded code runs.
    Run,
    /// Write `data` to RAM at `address`. Only ever issued while held.
    Write { address: u16, data: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptError {
    Empty,
    /// The data ends inside a record, or a length field is inconsistent.
    Malformed {
        record: usize,
    },
    /// A record is not of the one known shape.
    UnknownShape {
        record: usize,
    },
    /// A record asks for something other than a RAM write or a reset change.
    Unsafe {
        record: usize,
    },
    /// RAM would be written while the CPU is running.
    WriteWhileRunning {
        record: usize,
    },
    /// The script does not start by holding the CPU or does not end by releasing it.
    BadSequence,
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("the firmware script is empty"),
            Self::Malformed { record } => write!(f, "the firmware script is damaged at record {record}"),
            Self::UnknownShape { record } => {
                write!(f, "record {record} of the firmware script has a form this program does not know")
            }
            Self::Unsafe { record } => {
                write!(f, "record {record} of the firmware script is not a RAM write or a reset change")
            }
            Self::WriteWhileRunning { record } => {
                write!(f, "record {record} of the firmware script writes while the controller is running")
            }
            Self::BadSequence => f.write_str("the firmware script does not start with a hold and end with a run"),
        }
    }
}

impl std::error::Error for ScriptError {}

/// Parses a whole script. Every record is checked before anything is returned.
pub fn parse_script(bytes: &[u8]) -> Result<Vec<Step>, ScriptError> {
    let mut steps = Vec::new();
    let mut rest = bytes;
    let mut held = false;
    while !rest.is_empty() {
        let record = steps.len();
        let malformed = ScriptError::Malformed { record };
        let header = rest.get(..HEADER_LEN).ok_or(malformed.clone())?;
        let u32_at = |i: usize| u32::from_le_bytes([header[i], header[i + 1], header[i + 2], header[i + 3]]) as usize;
        if &header[..4] != TAG {
            return Err(malformed);
        }
        let (total, data_offset, data_len) = (u32_at(4), u32_at(8), u32_at(28));
        if data_offset != HEADER_LEN || total != HEADER_LEN + data_len || data_len > MAX_BLOCK {
            return Err(malformed);
        }
        let data = rest.get(HEADER_LEN..total).ok_or(malformed)?;
        rest = &rest[total..];

        let request = header[16];
        let address = u16::from_le_bytes([header[18], header[19]]);
        let index = u16::from_le_bytes([header[20], header[21]]);
        if header[12..16] != KNOWN_PREFIX || header[17] != KNOWN_FLAGS {
            return Err(ScriptError::UnknownShape { record });
        }
        if request != REQUEST_FIRMWARE_LOAD || index != 0 || data.is_empty() {
            return Err(ScriptError::Unsafe { record });
        }
        if address == CPU_CONTROL {
            match data {
                [1] => held = true,
                [0] => held = false,
                _ => return Err(ScriptError::Unsafe { record }),
            }
            steps.push(if held { Step::Hold } else { Step::Run });
        } else if usize::from(address) + data.len() <= RAM_END {
            if !held {
                return Err(ScriptError::WriteWhileRunning { record });
            }
            steps.push(Step::Write { address, data: data.to_vec() });
        } else {
            return Err(ScriptError::Unsafe { record });
        }
    }
    match (steps.first(), steps.last()) {
        (None, _) => Err(ScriptError::Empty),
        (Some(Step::Hold), Some(Step::Run)) => Ok(steps),
        _ => Err(ScriptError::BadSequence),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds one record of the known shape.
    fn record(request: u8, address: u16, index: u16, data: &[u8]) -> Vec<u8> {
        let mut r = Vec::new();
        r.extend_from_slice(TAG);
        r.extend_from_slice(&((HEADER_LEN + data.len()) as u32).to_le_bytes());
        r.extend_from_slice(&(HEADER_LEN as u32).to_le_bytes());
        r.extend_from_slice(&KNOWN_PREFIX);
        r.push(request);
        r.push(KNOWN_FLAGS);
        r.extend_from_slice(&address.to_le_bytes());
        r.extend_from_slice(&index.to_le_bytes());
        r.extend_from_slice(&[0; 6]);
        r.extend_from_slice(&(data.len() as u32).to_le_bytes());
        r.extend_from_slice(data);
        r
    }

    fn load(address: u16, data: &[u8]) -> Vec<u8> {
        record(REQUEST_FIRMWARE_LOAD, address, 0, data)
    }

    fn script(records: &[Vec<u8>]) -> Vec<u8> {
        records.concat()
    }

    #[test]
    fn a_two_stage_script_parses() {
        let s = script(&[
            load(CPU_CONTROL, &[1]),
            load(0x0100, &[1, 2, 3]),
            load(CPU_CONTROL, &[0]),
            load(CPU_CONTROL, &[1]),
            load(0x3ffd, &[9, 9, 9]),
            load(CPU_CONTROL, &[0]),
        ]);
        let steps = parse_script(&s).unwrap();
        assert_eq!(steps.len(), 6);
        assert_eq!(steps[1], Step::Write { address: 0x0100, data: vec![1, 2, 3] });
        assert_eq!(steps[2], Step::Run);
        assert_eq!(steps[3], Step::Hold);
    }

    #[test]
    fn anything_but_a_ram_write_is_refused() {
        let hold = load(CPU_CONTROL, &[1]);
        let run = load(CPU_CONTROL, &[0]);
        let wrap = |bad: Vec<u8>| script(&[hold.clone(), bad, run.clone()]);
        // Another vendor request, such as one that writes permanent storage.
        assert_eq!(parse_script(&wrap(record(0xA2, 0x0000, 0, &[1]))), Err(ScriptError::Unsafe { record: 1 }));
        // Past the end of RAM.
        assert_eq!(parse_script(&wrap(load(0x3fff, &[1, 2]))), Err(ScriptError::Unsafe { record: 1 }));
        // Another register.
        assert_eq!(parse_script(&wrap(load(0xE601, &[1]))), Err(ScriptError::Unsafe { record: 1 }));
        // Other bits of the CPU control register.
        assert_eq!(parse_script(&wrap(load(CPU_CONTROL, &[3]))), Err(ScriptError::Unsafe { record: 1 }));
        // A non-zero index.
        assert_eq!(
            parse_script(&wrap(record(REQUEST_FIRMWARE_LOAD, 0x0100, 1, &[1]))),
            Err(ScriptError::Unsafe { record: 1 })
        );
    }

    #[test]
    fn writes_need_the_cpu_held() {
        let s =
            script(&[load(CPU_CONTROL, &[1]), load(CPU_CONTROL, &[0]), load(0x0100, &[1]), load(CPU_CONTROL, &[0])]);
        assert_eq!(parse_script(&s), Err(ScriptError::WriteWhileRunning { record: 2 }));
    }

    #[test]
    fn the_sequence_must_start_held_and_end_running() {
        assert_eq!(parse_script(&[]), Err(ScriptError::Empty));
        assert_eq!(parse_script(&load(CPU_CONTROL, &[1])), Err(ScriptError::BadSequence));
        assert_eq!(parse_script(&load(CPU_CONTROL, &[0])), Err(ScriptError::BadSequence));
    }

    #[test]
    fn unknown_shapes_and_damage_are_refused() {
        let mut odd = load(CPU_CONTROL, &[1]);
        odd[17] = 0x71;
        assert_eq!(parse_script(&odd), Err(ScriptError::UnknownShape { record: 0 }));
        let good = script(&[load(CPU_CONTROL, &[1]), load(0x0100, &[1, 2, 3]), load(CPU_CONTROL, &[0])]);
        for n in 1..good.len() {
            // Every truncation is an error or a shorter valid script, never a panic.
            let _ = parse_script(&good[..n]);
        }
        let mut bad_tag = good.clone();
        bad_tag[0] = b'X';
        assert_eq!(parse_script(&bad_tag), Err(ScriptError::Malformed { record: 0 }));
    }
}
