# Contributing

Thank you for helping. AxeTwoDriver is young, and the most valuable
contributions right now are reports from real units and work on the items in
[`docs/gaps.md`](docs/gaps.md).

Start with [`AGENTS.md`](AGENTS.md); it is the short version of everything
here and applies to people as much as to AI agents.

## Set up

You need Windows 10 or 11 and [Rust](https://rustup.rs) 1.95 or newer with the
MSVC toolchain.

```powershell
git clone https://github.com/RamonWide/AxeTwoDriver
cd AxeTwoDriver
cargo test --workspace
```

The tests need no hardware. The pure crates (`axetwo-usb`, the `pcm` module)
also build and test on Linux and macOS; everything that talks to the unit is
Windows-only for now.

## Pull-request checklist

- `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`
  and `cargo test --workspace` pass.
- New parsing code has tests, including malformed input.
- New `unsafe` is in `axetwo-win` only and has a `SAFETY` comment.
- A new request to the unit is described in `spec/SPEC.md` first, with a tag.
- `docs/gaps.md`, `ROADMAP.md` and `CHANGELOG.md` are updated if what works
  has changed.
- Nothing is copied from Fractal Audio's software or from decompiler output
  ([`docs/clean-room.md`](docs/clean-room.md)).

## Hardware reports

If you own a unit, this is the most useful thing you can do. Run:

```powershell
axetwo probe
```

and open a **Hardware report** issue with the output, your model (II, II XL,
II XL+), firmware version and Windows version. `probe` only reads the unit's
descriptors. If you have also done the set-up, add the output of
`axetwo tone 5` and say whether `axetwo tone 3 -30` sounds clean.

## Commit messages

Short imperative subject, a body that says why. One logical change per
commit.
