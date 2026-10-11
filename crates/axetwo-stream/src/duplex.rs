//! The streaming loop.

use axetwo_usb::layout::{self, StreamLayout};
use axetwo_usb::{CLASS_AUDIO, PROTOCOL_AUDIO_2};
use axetwo_win::winusb::{self, Device, IsoQueue};
use axetwo_win::{hub, WinError};
use std::fmt;

/// Device interface class under which WinUSB exposes the unit's audio
/// function once `axetwo setup` has run.
pub const INTERFACE_GUID: &str = "{6B3F0E5C-2A8D-4E61-9C47-A2D15E0B7F31}";

/// Device instance-id prefix of the audio function of a running unit.
pub const AUDIO_FUNCTION_ID: &str = "USB\\VID_2466&PID_8003&MI_00\\";

/// Packets per transfer: one USB frame (1 ms) of microframes.
const PACKETS: usize = 8;
/// Capture transfers in flight.
const SLOTS: usize = 8;
/// Playback slots beyond the lead, so a slot has always completed before it is reused.
const SPARE_PLAYBACK_SLOTS: usize = 6;
/// Milliseconds of playback queued ahead of the unit unless set otherwise.
const DEFAULT_LEAD_MS: u32 = 2;
const MAX_LEAD_MS: u32 = 32;
/// Frames in each lead transfer: one millisecond at the nominal rate.
const NOMINAL_FRAMES: usize = 48;
/// Consecutive failed playback transfers after which the stream gives up.
const MAX_PLAYBACK_FAILURES: usize = 200;
const TIMEOUT_MS: u32 = 1000;

#[derive(Debug)]
pub enum StreamError {
    /// No running Axe-Fx II is attached.
    NotConnected,
    /// The unit is attached but waiting for its USB firmware.
    NeedsFirmware,
    /// The audio function is not on WinUSB yet.
    NotSetUp,
    Descriptor(axetwo_usb::ParseError),
    Layout(layout::LayoutError),
    Windows(WinError),
    /// The unit stopped sending audio.
    Stalled,
    /// Playback transfers keep failing.
    PlaybackFailed(u32),
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConnected => f.write_str("no Axe-Fx II is connected and switched on"),
            Self::NeedsFirmware => f.write_str("the Axe-Fx II is connected but its USB firmware has not been loaded"),
            Self::NotSetUp => f.write_str(
                "the unit's audio function is not set up for AxeTwoDriver; run `axetwo setup` as administrator",
            ),
            Self::Descriptor(e) => write!(f, "the unit's descriptors could not be read: {e}"),
            Self::Layout(e) => write!(f, "this unit is not supported: {e}"),
            Self::Windows(e) => write!(f, "{e}"),
            Self::Stalled => f.write_str("the unit stopped sending audio"),
            Self::PlaybackFailed(code) => {
                write!(f, "playback transfers keep failing (Windows error {code})")
            }
        }
    }
}

impl std::error::Error for StreamError {}

impl From<WinError> for StreamError {
    fn from(e: WinError) -> Self {
        Self::Windows(e)
    }
}

/// Returned by the processing callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Stop,
}

/// What happened during [`Duplex::run`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats {
    pub frames: u64,
    pub capture_packet_errors: u64,
    pub playback_transfer_errors: u64,
    /// Fewest and most frames the unit sent in one millisecond.
    pub frames_per_ms: Option<(usize, usize)>,
}

/// An opened unit, ready to stream.
pub struct Duplex {
    device: Device,
    layout: StreamLayout,
    control_interface: u8,
    lead_ms: u32,
}

impl Duplex {
    /// Finds the unit, checks that its descriptors are a layout this driver
    /// supports, and opens its audio function. Sends nothing but standard
    /// descriptor requests.
    pub fn open() -> Result<Self, StreamError> {
        let attached = hub::attached_devices()?;
        let fractal = |pid| attached.iter().find(move |d| d.vendor_id() == layout::VENDOR_ID && d.product_id() == pid);
        let Some(unit) = fractal(layout::PRODUCT_ID_RUNNING) else {
            return Err(if fractal(layout::PRODUCT_ID_BOOTLOADER).is_some() {
                StreamError::NeedsFirmware
            } else {
                StreamError::NotConnected
            });
        };
        let raw = hub::configuration_descriptor(&unit.hub_path, unit.port, 0)?;
        let config = axetwo_usb::parse_configuration(&raw).map_err(StreamError::Descriptor)?;
        let layout = layout::stream_layout(&config).map_err(StreamError::Layout)?;
        let control_interface = config
            .interfaces
            .iter()
            .filter(|i| i.class == CLASS_AUDIO && i.protocol == PROTOCOL_AUDIO_2)
            .map(|i| i.number)
            .min()
            .unwrap_or(0);
        let paths = winusb::interface_paths(INTERFACE_GUID)?;
        let path = paths.first().ok_or(StreamError::NotSetUp)?;
        let device = Device::open(path)?;
        Ok(Self { device, layout, control_interface, lead_ms: DEFAULT_LEAD_MS })
    }

    pub fn layout(&self) -> &StreamLayout {
        &self.layout
    }

    /// Sets how many milliseconds of playback are queued ahead of the unit
    /// when a stream starts (1 to 32). More rides out longer scheduling
    /// delays and adds the same amount of output latency.
    pub fn set_lead_ms(&mut self, ms: u32) {
        self.lead_ms = ms.clamp(1, MAX_LEAD_MS);
    }

    pub fn lead_ms(&self) -> u32 {
        self.lead_ms
    }

    /// Position of an interface within the function WinUSB opened.
    fn position(&self, interface: u8) -> usize {
        usize::from(interface.saturating_sub(self.control_interface))
    }

    /// Asks the unit's clock source for its current sampling frequency in Hz.
    pub fn sample_rate(&self) -> Result<u32, StreamError> {
        // Audio 2.0 GET CUR of CS_SAM_FREQ_CONTROL on the clock source entity.
        let index = (u16::from(self.layout.clock_id) << 8) | u16::from(self.control_interface);
        let bytes = self.device.control_in(0x21, 0x01, 0x0100, index, 4)?;
        match bytes.as_slice() {
            [a, b, c, d] => Ok(u32::from_le_bytes([*a, *b, *c, *d])),
            _ => Err(StreamError::Windows(WinError { what: "reading the sample rate", code: 0 })),
        }
    }

    /// Streams until `process` returns [`Flow::Stop`].
    ///
    /// `process` is called about once per millisecond with the frames the unit
    /// just sent (packed 24-bit samples, [`StreamLayout::capture`] channels)
    /// and a zeroed buffer for the same number of playback frames.
    pub fn run<F>(&self, mut process: F) -> Result<Stats, StreamError>
    where
        F: FnMut(&[u8], &mut [u8]) -> Flow,
    {
        let capture = self.position(self.layout.capture.interface);
        let playback = self.position(self.layout.playback.interface);
        self.device.set_alternate(capture, self.layout.capture.alternate)?;
        let result = self
            .device
            .set_alternate(playback, self.layout.playback.alternate)
            .map_err(StreamError::from)
            .and_then(|()| self.stream(capture, playback, &mut process));
        // Always return both interfaces to their idle setting.
        let a = self.device.set_alternate(playback, 0);
        let b = self.device.set_alternate(capture, 0);
        let stats = result?;
        a?;
        b?;
        Ok(stats)
    }

    fn stream(
        &self,
        capture: usize,
        playback: usize,
        process: &mut dyn FnMut(&[u8], &mut [u8]) -> Flow,
    ) -> Result<Stats, StreamError> {
        let in_frame = self.layout.capture.frame_bytes();
        let out_frame = self.layout.playback.frame_bytes();
        let in_bytes = usize::from(self.layout.capture.endpoint.max_packet) * PACKETS;
        let out_bytes = usize::from(self.layout.playback.endpoint.max_packet) * PACKETS;
        let max_out_frames = out_bytes / out_frame;
        let mut input =
            self.device.iso_queue(capture, self.layout.capture.endpoint.address, in_bytes, PACKETS, SLOTS)?;
        let slots = self.lead_ms as usize + SPARE_PLAYBACK_SLOTS;
        let mut output = Playback {
            queue: self.device.iso_queue(playback, self.layout.playback.endpoint.address, out_bytes, PACKETS, slots)?,
            next: 0,
            failures: 0,
            errors: 0,
        };
        for slot in 0..SLOTS {
            input.submit_read(slot)?;
        }
        let silence = vec![0u8; NOMINAL_FRAMES.min(max_out_frames) * out_frame];
        for _ in 0..self.lead_ms {
            output.send(&silence)?;
        }

        let mut stats = Stats::default();
        let mut captured = Vec::with_capacity(in_bytes);
        let mut to_play = vec![0u8; out_bytes];
        for n in 0usize.. {
            let slot = n % SLOTS;
            if input.wait(slot, TIMEOUT_MS)?.is_none() {
                return Err(StreamError::Stalled);
            }
            captured.clear();
            let data = input.data(slot)?;
            for p in input.packets(slot)? {
                if p.status != 0 {
                    stats.capture_packet_errors += 1;
                    continue;
                }
                let whole = p.length - p.length % in_frame;
                if let Some(bytes) = data.get(p.offset..p.offset + whole) {
                    captured.extend_from_slice(bytes);
                }
            }
            input.submit_read(slot)?;

            let frames = (captured.len() / in_frame).min(max_out_frames);
            stats.frames += frames as u64;
            stats.frames_per_ms = Some(match stats.frames_per_ms {
                Some((lo, hi)) => (lo.min(frames), hi.max(frames)),
                None => (frames, frames),
            });
            let out = to_play.get_mut(..frames * out_frame).unwrap_or(&mut []);
            out.fill(0);
            let flow = process(&captured, out);
            output.send(out)?;
            if flow == Flow::Stop {
                break;
            }
        }
        stats.playback_transfer_errors = output.errors;
        Ok(stats)
    }
}

struct Playback {
    queue: IsoQueue,
    next: usize,
    failures: usize,
    errors: u64,
}

impl Playback {
    /// Queues `bytes` as the next playback transfer.
    fn send(&mut self, bytes: &[u8]) -> Result<(), StreamError> {
        let slot = self.next % self.queue.slot_count();
        if self.next >= self.queue.slot_count() {
            // The slot was last used a full round ago; collect that result first.
            match self.queue.wait(slot, TIMEOUT_MS) {
                Ok(Some(_)) => self.failures = 0,
                Ok(None) => return Err(StreamError::Stalled),
                Err(e) => self.failed(e)?,
            }
        }
        self.next += 1;
        if let Some(dst) = self.queue.data_mut(slot)?.get_mut(..bytes.len()) {
            dst.copy_from_slice(bytes);
        }
        if let Err(e) = self.queue.submit_write(slot, bytes.len()) {
            self.failed(e)?;
        }
        Ok(())
    }

    fn failed(&mut self, e: WinError) -> Result<(), StreamError> {
        self.errors += 1;
        self.failures += 1;
        self.queue.restart();
        if self.failures > MAX_PLAYBACK_FAILURES {
            return Err(StreamError::PlaybackFailed(e.code));
        }
        Ok(())
    }
}
