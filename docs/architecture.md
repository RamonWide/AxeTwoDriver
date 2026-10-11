# Architecture

## The approach: no kernel code of our own

A classic Windows audio driver is a kernel driver, and Windows only loads
kernel drivers that Microsoft has signed. That is the wall the original 2014
driver ran into, and a new open-source kernel driver would face the same one:
getting a signature needs an extended-validation certificate and a Microsoft
hardware developer account.

AxeTwoDriver therefore uses only drivers that already ship with Windows, and
does the device-specific work in ordinary user-mode Rust:

```text
 DAW (ASIO host)                     any MIDI program
       |                                    |
 AxeTwoDriver-DAW library            Windows MIDI
       |                                    |
 axetwo-stream   <- implicit feedback       |
       |                                    |
 WinUSB (Microsoft)              USB audio class driver (Microsoft)
       |                                    |
       +--------- USB composite driver (Microsoft) ---------+
                              |
                         Axe-Fx II
```

- **MIDI** works with Windows' own class driver once the original audio
  driver is out of the way. Nothing of ours is involved.
- **Audio** cannot use Windows' USB Audio 2.0 driver, which refuses the unit
  (see `spec/SPEC.md`, section 3). `axetwo setup` puts Microsoft's WinUSB
  driver on the audio function instead, and `axetwo-stream` exchanges audio
  through it.

`axetwo setup` installs no driver package. It selects `winusb.inf`, which is
part of Windows and signed by Microsoft, for one device node, as Device
Manager's "Let me pick from a list" does. No certificate is added to the
machine.

## Crates

```text
crates/
  axetwo-usb     L0  descriptor parsing and the supported stream layout (pure, no unsafe)
  axetwo-win     L1  Windows system calls: hub queries, device set-up, WinUSB (the only unsafe)
  axetwo-stream  L2  the full-duplex streaming loop; PCM helpers
apps/
  axetwo             command-line tool: status | probe | setup | capture | tone
  axetwo-panel       control panel: status, USB streaming mode, buffer size
  axetwo-setup       installer: original driver off (with backup), WinUSB on, files, registration
  axetwo-loader      background program: sends the USB firmware when the unit is switched on
spec/                functional specification
```

A crate may depend only on lower layers. `unsafe` is denied workspace-wide
and allowed in `axetwo-win` alone, where every block carries a `SAFETY`
comment.

## The streaming loop

The unit is the clock master. `axetwo-stream` keeps eight capture transfers
of one millisecond each in flight. When one completes it counts the frames
that arrived, hands them to the caller with an empty playback buffer of the
same number of frames, and queues that buffer as the next playback transfer.
Two playback transfers are queued before the loop starts, so the unit always
has about two milliseconds in hand.

Before any of that, the unit's descriptors are read and checked against the
layout the code supports; an unexpected device is refused with a message
instead of being streamed to.

### A WinUSB limitation

WinUSB splits each playback transfer into packets itself, filling every
packet to the endpoint's maximum before starting the next. A millisecond of
48 frames therefore goes out as six packets of 7 frames and one of 6, not as
eight packets of 6. The total per millisecond is right, and the unit reports
no errors, but the distribution inside the millisecond is burstier than a
kernel driver would produce. Whether that is audible is the first open item
in `docs/gaps.md`.

## The ASIO driver: AxeTwoDriver-DAW

An ASIO driver is a user-mode library loaded by the DAW, so it needs no
kernel signature either. It sits on `axetwo-stream` and re-blocks the
millisecond transfers into the DAW's buffer size. Still to come are the streaming
modes and the control panel listed in `spec/SPEC.md`, section 4. Its name avoids "ASIO" because Steinberg's usage guidelines do not allow the mark in product names.

It is kept apart from the rest of the project, for licensing:

| Part | Where | License |
|---|---|---|
| Everything in this repository | `axetwo-usb`, `axetwo-win`, `axetwo-stream`, `axetwo` | MIT |
| The ASIO driver library | its own repository, `AxeTwoDriver-DAW`, depending on the crates above | GPLv3, written against Steinberg's ASIO SDK under the SDK's GPLv3 option |

The MIT crates know nothing about ASIO and can be used by any other audio
front end. The ASIO library is a thin adapter around them. MIT code may be
combined into a GPLv3 work, so the adapter can use the crates as they are.
For users the split is invisible: a release bundles both, and one set-up
command registers the library with Windows.

What this approach cannot give is a Windows sound device (for browsers, media
players and system sounds). That needs a kernel driver; see `ROADMAP.md`.
