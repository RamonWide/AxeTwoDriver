# Attribution

Everything in this repository was written for AxeTwoDriver, with the
exceptions listed here.

## Data that originates outside this project

| What | Where | Origin |
|---|---|---|
| USB configuration descriptor of an Axe-Fx II XL | `crates/axetwo-usb/src/layout.rs` (test data) | Read from the device with `axetwo probe`. It is the device's public self-description, sent to any computer it is plugged into. |
| USB vendor and product IDs `2466:0003`, `2466:8003` | `crates/axetwo-usb/src/layout.rs` | Reported by the device. Facts required to find it. |
| Names and values of the original driver's settings | `spec/SPEC.md`, section 4; the list of seven USB streaming modes in `crates/axetwo-stream/src/settings.rs` | Short functional names and the number of milliseconds each stands for, recovered by analysing the original driver and kept so that users find the settings they know. Facts and short labels; no license claimed. See `docs/clean-room.md`. |

No file from Fractal Audio's software is included.

## Rust dependencies

Fetched by Cargo at build time, not vendored. `Cargo.lock` pins the exact
versions.

| Crate | License |
|---|---|
| `windows-sys` and its `windows-*` support crates | MIT OR Apache-2.0 |
| `eframe`, `egui` and their dependencies (control panel only) | MIT OR Apache-2.0, with a few other permissive licenses |

The control panel also embeds the default fonts that ship with `egui`
(Ubuntu Light, Hack, Noto Emoji, emoji-icon-font), each under its own open
license as distributed by `epaint_default_fonts`.

`scripts/third-party.py` collects the license text of every crate that goes
into the Windows programs; the download carries the result as
`THIRD-PARTY-LICENSES.txt`.

The download may also contain `axetwo_daw.dll` from the separate
[AxeTwoDriver-DAW](https://github.com/RamonWide/AxeTwoDriver-DAW) project, which is
licensed under the GNU General Public License version 3. It is a separate
program shipped alongside; nothing in this repository links to it.
