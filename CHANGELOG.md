# Changelog

All notable changes to AxeTwoDriver are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `axetwo probe`: reads and decodes the unit's USB descriptors through the hub
  driver, with no driver bound to the unit.
- `axetwo status`: says what is connected and whether it is ready to stream.
- `axetwo setup`: puts Windows' own WinUSB driver on the unit's audio
  function, without a driver package or certificate.
- `axetwo capture` and `axetwo tone`: full-duplex streaming test, 4 channels
  in and 2 out at 24-bit 48 kHz, with implicit feedback.
- Functional specification of the unit on USB (`spec/SPEC.md`).
- Control panel (`axetwo-panel`): device status, USB streaming mode (1 to
  32 ms) and suggested buffer size, stored per user.
- Installer (`axetwo-setup`): one double-click backs up and removes the
  original audio driver, sets up audio, installs the programs and registers
  the optional DAW library; uninstalls from Installed apps.
- `scripts/package.ps1` builds the download.

[Unreleased]: https://github.com/RamonWide/AxeTwoDriver/commits/main
