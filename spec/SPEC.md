# Axe-Fx II on USB: functional specification

What AxeTwoDriver knows about how an Axe-Fx II behaves on USB and what the
original Windows driver offers. Code in this repository is written from this
document and from the public USB specifications, not from the original
driver's code.

Every fact carries a tag saying how it is known:

| Tag | Meaning |
|---|---|
| `[H]` | Observed on hardware with this project's own tools |
| `[D]` | Read from the device's own USB descriptors |
| `[W]` | Reported by Windows (Device Manager, event log) |
| `[B]` | Found by analysing the original driver's files |
| `[P]` | Public documentation, not verified here |
| `[?]` | Unknown or a guess |

Verified on: Axe-Fx II XL, firmware 32.00, Windows 11 Pro build 26200, xHCI
host port.

## 1. Two identities

| State | VID:PID | What it is |
|---|---|---|
| Waiting for USB firmware | `2466:0003` | A loader target. The original package's loader driver sends it a firmware image, after which it disconnects and returns as the second identity. `[W]` |
| Running | `2466:8003` | A composite device: Audio Class 2.0 function plus a USB-MIDI function. `[D]` |

After power-on the unit appears as `0003` and needs the firmware on every
power cycle. `[W]` (first arrival as `0003`, then `8003` two seconds later).

### 1.1 Firmware loading

Not implemented by this project yet; the original loader still does it.

- The original loader takes its instructions from a script file of records.
  Each record starts with the ASCII tag `CSPT`, a 32-bit length, and describes
  one control transfer with its data. `[B]`
- In the script examined, every record is vendor request `0xA0` (the Cypress
  EZ-USB "firmware load" request), with the target address in `wValue`. The
  first record writes `01` to address `0xE600` and the last writes `00` there
  (hold the controller in reset, then release it). `[B]`
- Two different script files exist on a machine with the original packages
  installed (6991 and 7104 bytes). Which the loader uses is `[?]`.
- Whether the image is only loaded to RAM (lost at power-off) or anything is
  written to persistent storage has not been established: `[?]`. Until it is,
  nothing in this repository sends request `0xA0`.

## 2. The running device

Device descriptor: USB 2.00, class `EF/02/01` (interface association),
`bMaxPacketSize0` 64, manufacturer "Fractal Audio Systems", product
"AXE-FX II", no serial number, 2 configurations. `[D]`
Both configurations are byte-for-byte identical (303 bytes). `[D]`
High speed. `[H]`

| Interface | Class | What |
|---|---|---|
| 0 | Audio 2.0, AudioControl | Clock source and terminals |
| 1 | Audio 2.0, AudioStreaming | Playback: computer to unit |
| 2 | Audio 2.0, AudioStreaming | Capture: unit to computer |
| 3 | Audio 1.0, AudioControl | Header for the MIDI interface |
| 4 | Audio 1.0, MIDIStreaming | MIDI in and out |

Interfaces 0 to 2 are grouped by an interface association; 3 and 4 are not.
`[D]`

### 2.1 Clock

One clock source, ID `0x28`: internal, fixed. Its frequency control is
readable only. `[D]` It reports 48000 Hz. `[H]`

Reading it: class request to the interface, `bmRequestType 0xA1`,
`bRequest 0x01` (CUR), `wValue 0x0100` (sampling frequency), `wIndex 0x2800`
(clock ID in the high byte, interface 0 in the low byte), 4 bytes,
little-endian Hz. `[H]`

### 2.2 Streams

| | Playback | Capture |
|---|---|---|
| Interface, alternate setting | 1, alt 1 | 2, alt 1 |
| Endpoint | `0x02` OUT | `0x86` IN |
| Transfer | isochronous, asynchronous | isochronous, asynchronous, implicit feedback |
| Interval | every microframe (125 Âµs) | every microframe |
| Max packet | 42 bytes | 84 bytes |
| Channels | 2 | 4 |
| Sample | 24-bit PCM in 3 bytes, little-endian | the same |
| Frame | 6 bytes | 12 bytes |

All `[D]`. Alternate setting 0 of each streaming interface has no endpoint
(stream stopped). `[D]`

The playback endpoint is asynchronous but the device declares **no feedback
endpoint**. `[D]` The capture endpoint is marked as an implicit-feedback data
endpoint instead: the host learns the unit's sample rate from how many frames
arrive and must send the same number. `[D]`

Observed while streaming: `[H]`

- Capture packets carry 72 bytes (6 frames) per microframe, 48 frames per
  millisecond; about once a second a millisecond carries 49 frames. The unit's
  clock is therefore independent of the host's and slightly faster on the unit
  tested.
- Sending back exactly as many playback frames as arrived in each millisecond
  ran for several seconds with no transfer errors in either direction.
- With no instrument playing, capture channels 1 and 2 sit near -112 dBFS and
  channels 3 and 4 are digital silence.
- A sine sent to playback does not appear on any capture channel with the
  unit's default routing.
- A 440 Hz sine at -30 dBFS sent to playback is heard from the unit as a
  clean, steady tone. This holds although WinUSB packs each millisecond of
  playback into full 7-frame packets followed by a shorter one, not 6 frames
  in every microframe (see `docs/architecture.md`).

Not yet established: `[?]`

- What the four capture channels carry in each of the unit's I/O
  configurations, and where the two playback channels go.

### 2.3 MIDI

Interface 4 is a standard USB-MIDI 1.0 streaming interface: one embedded and
one external jack each way, bulk endpoints `0x04` OUT and `0x81` IN, 512
bytes. `[D]`

Windows' own class driver runs it without any vendor driver, and the unit
answers SysEx over it. `[H]`

## 3. Why Windows refuses to run the unit

- The original audio driver (version 2.23.0.0, dated 2014) is not started:
  device problem 52, status `0xC0000428` (the driver image's signature is not
  accepted). `[W]`
- With the original audio driver removed, Windows binds its own drivers: the
  composite driver for the device, its USB Audio 2.0 driver for interfaces 0
  to 2, and its USB audio driver for the MIDI function. `[W]`
- The MIDI function starts. `[W]`
- The Audio 2.0 function does not: problem 10, and the driver logs "The driver
  could not find a feedback endpoint for an asynchronous data OUT endpoint".
  `[W]` Windows' driver supports explicit feedback only. `[P]`

So MIDI needs no driver of ours, and audio needs something that honours
implicit feedback.

## 4. What the original driver offers

The original audio driver is a rebranded build of a generic, commercial USB
Audio 2.0 class driver, not written for this unit in particular. `[B]` Nothing
vendor-specific to the Axe-Fx was found in what has been examined so far;
that examination is incomplete.

User-visible features to match:

| Feature | Detail | Tag |
|---|---|---|
| ASIO driver | Registered as "FAS USB Audio ASIO Driver", in 32-bit and 64-bit form | `[B]` |
| Windows sound device | A kernel-streaming device, so the unit appears among Windows' playback and recording devices | `[B]` |
| MIDI ports | Created by the same driver | `[B]` |
| Control panel | Pages: status and current sample rate, stream formats, clock source, buffer settings, volume, info | `[B]` |
| USB streaming mode | Seven choices: Minimum Latency, Low Latency, Standard, Relaxed, Reliable, Safe, Extra Safe | `[B]` |
| ASIO buffer size | A list of sizes in samples, with a warning when the size is too small for the streaming mode | `[B]` |

Settings model: `[B]`

- The streaming mode is stored as a number of milliseconds of USB buffering.
  The seven accepted values are 1, 2, 4, 8, 12, 16 and 32. Their order matches
  the seven names above; the pairing of each name with each value is by that
  order and is `[?]` until checked in the control panel.
- The ASIO buffer size is stored in samples; any value from 16 to 65536 is
  accepted by the driver. Which sizes the control panel lists is `[?]`.
- A separate switch selects between two ways of choosing the ASIO buffer
  size (values 0 and 1); what each means is `[?]`.
- Two switches allow several ASIO programs, or ASIO and the Windows sound
  device, to use the unit at once.

Start-up sequence, in outline: `[B]` choose the stream formats, read the
current sample rate from the clock source, prepare the streams, the Windows
sound device and the ASIO side, apply the sample rate to the stream engine,
then start streaming. With this unit's fixed clock that involves no write to
the device beyond selecting the configuration and the streaming alternate
settings. `[?]` (inferred from the read-only clock; not captured on the wire).

Still to be analysed: how the original sizes its playback packets under
implicit feedback; the ASIO sample format, channel names and reported
latencies; the list of ASIO buffer sizes; the control panel's exact behaviour.
