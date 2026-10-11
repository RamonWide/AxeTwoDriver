## What and why

<!-- What does this change, and what problem does it solve? Link the issue if there is one. -->

## How it was tested

<!-- Tests added or changed. If you tried it on a real unit: model, firmware, Windows version. -->

## Checklist

- [ ] `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` pass
- [ ] New `unsafe` is in `axetwo-win` only and has a `SAFETY` comment
- [ ] A new request to the unit is described in `spec/SPEC.md` first, with a tag
- [ ] `docs/gaps.md`, `ROADMAP.md` and `CHANGELOG.md` are updated if what works has changed
- [ ] Nothing here is copied from Fractal Audio's software or from decompiler output (`docs/clean-room.md`)
