# Changelog

All notable changes to AxeTwoDriver are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-10-11

The first release. Experimental: installed and used on one computer with one
Axe-Fx II XL.

### Added

- USB MIDI on current Windows, by handing the unit to Windows' own class
  driver.
- Full-duplex audio through Microsoft's WinUSB driver, with no kernel code
  and no certificate: 4 channels in and 2 out at 24-bit 48 kHz, timed by
  implicit feedback.
- Installer (`axetwo-setup`): one double-click backs up and removes the
  original audio driver, sets up audio, installs the programs and registers
  the optional DAW library from
  [AxeTwoDriver-DAW](https://github.com/RamonWide/AxeTwoDriver-DAW);
  uninstalls from Installed apps.
- Control panel (`axetwo-panel`): device status, USB streaming mode (1 to
  32 ms) and suggested buffer size, stored per user.
- Command-line tool (`axetwo`): `status`, `probe` (reads and decodes the
  unit's USB descriptors with no driver bound), `setup`, `capture` and `tone`.
- Functional specification of the unit on USB (`spec/SPEC.md`).

[Unreleased]: https://github.com/RamonWide/AxeTwoDriver/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/RamonWide/AxeTwoDriver/releases/tag/v0.1.0
