# Clean-room rules

AxeTwoDriver is an independent project that interoperates with hardware made
by Fractal Audio Systems. It is written from a functional specification and
the public USB specifications, not from the original driver's code. This page
explains how that is kept true, so that every contributor works the same way.

## Sources, in order of preference

1. **The device itself.** Its USB descriptors say almost everything needed,
   and `axetwo probe` prints them.
2. **Public specifications**: USB 2.0, the USB Audio Class 2.0 and USB-MIDI
   device class definitions, and Microsoft's documentation for WinUSB.
3. **What Windows reports**: Device Manager, the event log.
4. **Analysis of the original driver**, only for behaviour the first three
   cannot show, such as which settings the original offers its users.

## Two roles, two sets of material

| Role | May read | Produces |
|---|---|---|
| **Analyst** | The original driver's files, disassembly and decompiler output, USB captures | `spec/SPEC.md`: prose and tables describing behaviour, each fact tagged with its source |
| **Implementer** | `spec/`, public documentation, this repository | Code, tests, docs |

An implementer does not open decompiler output. An analyst does not paste
code or pseudocode into the spec. Analysis material lives outside this
repository and is never committed; `.gitignore` excludes the folder names
used for it.

### How the first version was made

The first version of this repository was produced by a single AI agent in one
session. The streaming code was written and verified on hardware from the
device's descriptors and the public specifications before the original driver
was opened in an analysis tool. The same session then analysed parts of the
original driver to document its user-visible settings (`spec/SPEC.md`,
section 4, facts tagged `[B]`). Two pieces of code follow from that
analysis: the list of seven USB streaming modes, their names and their
values in milliseconds, in `crates/axetwo-stream/src/settings.rs`; and the
record layout of the original firmware script that
`crates/axetwo-usb/src/loader.rs` reads, which was worked out from the
script file itself. Nothing else was written from it, and no code was translated from decompiler
output, but the two roles were not held by separate people. From here
on, keep the roles in separate sessions or with separate people.

## Never in this repository

- Any file from Fractal Audio's software: drivers, INF files, firmware or
  loader scripts, control-panel files, installers.
- The unit's USB firmware in any form, including in tests. The loader reads
  it from the user's own installed copy of the original package.
- Decompiler or disassembler output, or code translated from it.
- The original's user-interface text beyond the short names of settings.

The device's USB descriptors are in the tests. They are the device's public
self-description, readable by any computer it is plugged into.

## Names and trademarks

"Axe-Fx" and "Fractal Audio" are trademarks of Fractal Audio Systems. "ASIO"
is a trademark of Steinberg Media Technologies. They are used in this project
only to say what the software works with. AxeTwoDriver is not affiliated
with, sponsored by or endorsed by either company.

## Third-party material

Anything not written for this project needs a permissive license, its license
file next to it, and a row in `ATTRIBUTION.md` in the same change.
