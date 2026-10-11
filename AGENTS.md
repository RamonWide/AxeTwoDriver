# AGENTS.md: guide for AI agents and contributors

AxeTwoDriver makes Fractal Audio's Axe-Fx II family (II, II XL, II XL+) usable
over USB on current Windows, in **Rust only** and without kernel code. It is
an independent, clean-room project: not affiliated with or endorsed by Fractal
Audio Systems.

The product name is written **AxeTwoDriver**. Machine names are lowercase:
crates `axetwo-*`, binary `axetwo`.

Read this file first, then `docs/`.

## 1. Orientation

| Read | Why |
|---|---|
| `docs/architecture.md` | Why there is no kernel driver, the crate layers, the streaming loop |
| `docs/clean-room.md` | What you may and may not look at or copy. **Read before writing code.** |
| `spec/SPEC.md` | What is known about the unit on USB and about the original driver, with confidence tags |
| `docs/gaps.md` | Known shortfalls, ranked: the work list to pick from |
| `ROADMAP.md` | Where the project stands against the original driver, and the milestones |
| `docs/install.md` | The steps a user follows today |
| `CONTRIBUTING.md` | Set-up, the pull-request checklist, hardware reports |

## 2. Workspace map

```text
crates/
  axetwo-usb     L0  descriptor parsing, supported stream layout (pure; no workspace deps)
  axetwo-win     L1  Windows system calls: hub queries, set-up, WinUSB (the only crate with unsafe)
  axetwo-stream  L2  full-duplex streaming loop, PCM helpers
apps/
  axetwo             command-line tool
  axetwo-panel       control panel (egui): status and buffer settings
  axetwo-setup       installer and uninstaller
scripts/package.ps1  builds the download
packaging/           files that go into the download
spec/                functional specification (prose and tables only)
docs/                architecture, install guide, gaps, clean-room rules
.github/             CI, issue and pull-request templates
```

A crate may depend only on lower layers.

## 3. Golden rules

1. **Hardware is someone's instrument.** Never send a request to the unit
   that the spec does not describe. Read before you write: check the unit's
   descriptors, then act. The firmware loader may only ever send what
   `axetwo_usb::loader` accepts: RAM writes and reset changes, verified by
   read-back. Do not widen that parser. Automated tests never touch
   hardware.
2. **Never crash.** A device that answers garbage, or not at all, must
   produce an error the user can act on. Non-test code has no `unwrap`,
   `expect`, `panic!`, `todo!` or unchecked indexing on device-derived values.
3. **`unsafe` stays in `axetwo-win`**, behind safe functions that return
   owned data, and every block has a `SAFETY` comment.
4. **Clean-room.** Implement from `spec/SPEC.md` and public specifications
   only. Do not read decompiler output while writing code, and never copy
   files from Fractal Audio's software. See `docs/clean-room.md`.
5. **Evidence over guesses.** Facts in the spec carry a tag: `[H]` hardware,
   `[D]` descriptors, `[W]` reported by Windows, `[B]` analysis of the
   original, `[P]` public docs, `[?]` unknown. Upgrading a tag needs
   evidence. If you build on a `[?]`, list it in `docs/gaps.md`.
6. **No system change without the user asking.** Anything that alters which
   driver Windows uses for a device is an explicit command, says what it
   changes, and says how to undo it.

## 4. Before you open a pull request

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## 5. Trying it on a unit

```sh
axetwo status      # read-only
axetwo probe       # read-only
axetwo tone 1      # streams silence for a second and reports errors
```

`axetwo setup` changes the driver Windows uses for the unit's audio function
and needs an administrator prompt; do not run it on someone's machine without
asking them.
