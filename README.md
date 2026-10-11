<h1 align="center">AxeTwoDriver</h1>

<p align="center">
  <b>An open-source USB driver for the Axe-Fx II on current Windows, rebuilt from scratch in pure Rust.</b><br>
  No kernel code, no certificates to trust: Windows' own drivers underneath,<br>
  and the part that is specific to the unit in ordinary, readable Rust.
</p>

<p align="center">
  <img alt="100% Rust" src="https://img.shields.io/badge/100%25-Rust-b7410e?style=flat-square&logo=rust">
  <img alt="Windows 10 and 11" src="https://img.shields.io/badge/Windows-10%20%C2%B7%2011-2f7bf5?style=flat-square">
  <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-3a3a3a?style=flat-square">
  <img alt="Status: experimental" src="https://img.shields.io/badge/status-experimental-d69e2e?style=flat-square">
  <a href="https://github.com/RamonWide/AxeTwoDriver/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/RamonWide/AxeTwoDriver/ci.yml?branch=main&style=flat-square&label=CI"></a>
</p>

<p align="center">
  <a href="#where-it-stands">Status</a> ·
  <a href="#why-the-original-stopped-working">Why</a> ·
  <a href="#how-it-works">How it works</a> ·
  <a href="#install">Install</a> ·
  <a href="ROADMAP.md">Roadmap</a> ·
  <a href="CONTRIBUTING.md">Contributing</a>
</p>

> [!IMPORTANT]
> **Status: experimental.** Today AxeTwoDriver gets the unit's USB **MIDI** working on Windows 11
> and gives a DAW an audio device through the companion project
> [AxeTwoDriver-DAW](https://github.com/RamonWide/AxeTwoDriver-DAW). It adds no Windows sound device,
> and it contains no firmware: Fractal Audio's own driver package must be installed, because the
> unit's USB firmware is taken from it. It has been run on
> one Axe-Fx II XL on one computer. [ROADMAP.md](ROADMAP.md) says exactly where things stand.

## Where it stands

| | State |
|---|---|
| USB MIDI (editors, librarians, MIDI control) | **Works**, using Windows' own class driver |
| Audio capture, 4 channels, 24-bit, 48 kHz | **Works** in the test tool, verified on hardware |
| Audio playback, 2 channels | **Works**: a test tone plays cleanly on the unit |
| ASIO driver for DAWs | **Works** in one DAW so far, from [AxeTwoDriver-DAW](https://github.com/RamonWide/AxeTwoDriver-DAW); 64-bit hosts |
| Control panel | Status, USB streaming mode, buffer size |
| Installer | One download, one double-click; uninstalls from Installed apps |
| Windows sound device (browser, media players) | Not possible without a kernel driver; not planned |
| USB firmware loading at power-on | Own user-mode loader; it sends the firmware the original package installed and ships none itself |

## Why the original stopped working

The original audio driver dates from 2014. Current Windows refuses to start it because it no
longer accepts the driver's signature (Device Manager shows code 52). With that driver removed,
Windows tries its own USB Audio 2.0 driver, which runs the unit's MIDI but rejects its audio: the
unit times its playback stream by *implicit feedback*, which Microsoft's driver does not support.

The details, each tagged with how it was verified, are in [`spec/SPEC.md`](spec/SPEC.md).

## How it works

A new kernel driver would hit the same signing wall as the old one. AxeTwoDriver avoids kernel
code entirely:

- **MIDI** is left to Windows' own USB audio class driver.
- **Audio** goes through **WinUSB**, a generic driver that ships with Windows and is signed by
  Microsoft. `axetwo setup` selects it for the unit's audio function; nothing is installed and no
  certificate is added.
- The streaming itself, including implicit feedback, is user-mode Rust in
  [`axetwo-stream`](crates/axetwo-stream). An ASIO driver is a user-mode library too, so the
  planned DAW support needs no kernel signature either.

More in [`docs/architecture.md`](docs/architecture.md).

## Install

Download the zip from the [latest release](https://github.com/RamonWide/AxeTwoDriver/releases/latest),
unzip it, and double-click `axetwo-setup.exe`. The guide is [`docs/install.md`](docs/install.md).
Fractal Audio's own driver package must be installed first: the unit's USB firmware comes from it.

## Build from source

You need [Rust](https://rustup.rs) 1.95 or newer and Windows 10 or 11.

```powershell
git clone https://github.com/RamonWide/AxeTwoDriver
cd AxeTwoDriver
cargo build --release
.\target\release\axetwo status
```

`scripts\package.ps1` builds the download, including the DAW library when its checkout sits next
to this one. The command-line tool has a few test commands:

```powershell
axetwo probe                  # the unit's USB descriptors, decoded
axetwo capture 5 take.wav     # record five seconds from the unit
axetwo tone 3 -30             # play a quiet test tone to it
```

## Under the hood

- **Three small crates in layers:** descriptor parsing, Windows system calls, the streaming loop;
  the command-line tool, control panel and installer are thin programs on top.
- **`unsafe` in one place:** only `axetwo-win`, which calls the operating system, may use it, and
  every block says why it is sound.
- **Checked before streaming:** the unit's descriptors are parsed and compared with the layout the
  code supports; anything unexpected is refused with a message.
- **Never crash:** no `unwrap`, `expect` or `panic!` outside tests.
- **Clean-room:** written from the device's own descriptors, the public USB specifications and a
  functional spec ([`docs/clean-room.md`](docs/clean-room.md)). No file from the original driver
  is in this repository.

## Documentation

| For | Read |
|---|---|
| Installing and first steps | [docs/install.md](docs/install.md) |
| Where the project stands | [ROADMAP.md](ROADMAP.md), [docs/gaps.md](docs/gaps.md), [CHANGELOG.md](CHANGELOG.md) |
| How it is built | [docs/architecture.md](docs/architecture.md), [spec/SPEC.md](spec/SPEC.md), [docs/clean-room.md](docs/clean-room.md) |
| Contributing | [CONTRIBUTING.md](CONTRIBUTING.md), [AGENTS.md](AGENTS.md), [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) |
| Security | [SECURITY.md](SECURITY.md) |

Looking for an editor? [Axe-Grid](https://github.com/RamonWide/axe-grid) is the sister project.

---

## License and credits

AxeTwoDriver is released under the [MIT License](LICENSE). Copyright (c) 2026 The AxeTwoDriver
contributors. Every file not written for this project is listed in [ATTRIBUTION.md](ATTRIBUTION.md).

<sub>Axe-Fx and Fractal Audio are trademarks of Fractal Audio Systems, LLC. ASIO is a trademark of Steinberg Media Technologies GmbH. AxeTwoDriver is an independent, open-source project and is not affiliated with, sponsored by or endorsed by either company; these names are used only to describe what it works with. AxeTwoDriver comes with no warranty: you use it with your own equipment at your own risk.</sub>
