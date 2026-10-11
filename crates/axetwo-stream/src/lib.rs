//! Full-duplex audio streaming with an Axe-Fx II.
//!
//! The unit is the clock master. Each millisecond it sends some number of
//! frames (48 on average at 48 kHz); the host answers with exactly that many
//! playback frames. This is what the USB Audio specification calls implicit
//! feedback, and it is the whole of the rate control.

pub mod pcm;
pub mod settings;

#[cfg(windows)]
mod duplex;
#[cfg(windows)]
pub mod status;
#[cfg(windows)]
pub use duplex::{Duplex, Flow, Stats, StreamError, AUDIO_FUNCTION_ID, INTERFACE_GUID};
