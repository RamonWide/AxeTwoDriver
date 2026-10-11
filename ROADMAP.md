# Roadmap

Last reviewed: 2026-10-11.

The goal is that an Axe-Fx II owner on current Windows gets back what the
original driver gave them: MIDI ports, an ASIO device for the DAW, a Windows
sound device, and the settings they are used to. `spec/SPEC.md`, section 4,
lists those features.

## Against the original driver

| Feature of the original | AxeTwoDriver |
|---|---|
| MIDI in and out | Works (Windows' own driver) |
| ASIO driver, 4 in / 2 out, 24-bit, 48 kHz | Experimental in AxeTwoDriver-DAW (64-bit hosts); not yet tried in a DAW |
| USB streaming modes (1 to 32 ms) | Works: the same seven modes, set in the control panel |
| ASIO buffer size choice | 64 to 4096 frames, in the control panel or the host |
| Several ASIO programs at once | Not yet |
| Windows sound device | Not possible in user mode |
| Control panel | Status, format and buffer settings; no clock-source or volume pages (the unit has neither control) |
| Firmware loading at power-on | Uses the original loader |
| Installer | `axetwo-setup.exe`: one double-click; uninstall from Installed apps |

## Milestones

### 0.1: proof on hardware (now)

- [x] Read and decode the unit's descriptors without any driver bound
- [x] MIDI through Windows' own driver
- [x] WinUSB on the audio function without a custom driver package
- [x] Full-duplex streaming with implicit feedback, verified on an Axe-Fx II XL
- [x] Playback confirmed by ear
- [ ] A one-hour stream without errors

### 0.2: ASIO

- [x] ASIO driver library on top of `axetwo-stream`, 64-bit, in its own GPLv3
      repository, AxeTwoDriver-DAW (see `docs/architecture.md`)
- [x] One download and one double-click install both parts
- [x] Buffer sizes and streaming modes as in the original, with a control panel
- [x] Pro-audio thread priority for the streaming loop (in the DAW library)
- [x] The installer removes the original audio driver itself, with a backup, and has an uninstaller
- [x] Used in one DAW
- [ ] Tested in a second DAW, and by a second person on a second machine
- [ ] A release workflow that builds and publishes the download

### 0.3: standing on its own

- [ ] Firmware loader, reading the firmware from the user's installed original package
- [ ] 32-bit ASIO driver for older hosts
- [ ] Reports from the original Axe-Fx II and the XL+

### Later, and only with help

- A Windows sound device. This needs a kernel driver and therefore a signing
  certificate and a Microsoft hardware developer account. If someone can
  sponsor that, a small kernel audio driver in Rust becomes realistic, and it
  would also remove the WinUSB packet-pattern limitation described in
  `docs/architecture.md`.
