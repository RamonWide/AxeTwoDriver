# Known gaps

Ranked by how much each one blocks real use. Tags are those of `spec/SPEC.md`.

1. **One DAW, one machine.** The DAW library has been used in one DAW by
   the project's owner: it opens and plays cleanly. Recording through it,
   other hosts, long sessions and small buffers under load are untested.
2. **Playback has only been judged by ear, briefly.** A 440 Hz test tone
   sounds clean; nobody has measured distortion or listened to music through
   it for long.
3. **No Windows sound device.** Not possible without a kernel driver.
4. **AxeTwoDriver's own firmware loader has not yet loaded a real unit.**
   The script is known to write RAM only (`spec/SPEC.md`, 1.1) and the
   loader verifies every block, but until it has been run at a power-on the
   original loader is the proven one. The firmware itself still comes from
   the original package, which therefore must stay installed.
5. **Parts of the installer have only run as no-ops.** On the one machine
   it was tried on, the original driver was already removed and audio was
   already set up, so those two steps reported "nothing to do". The code
   under them is the same that was run by hand earlier, but not through the
   installer. The uninstaller has not been run at all.
6. **The installer is not signed.** Windows SmartScreen will warn about it.
7. **One unit, one computer.** Everything was verified on a single Axe-Fx II
   XL on one Windows 11 machine with an xHCI port. The original II and the
   XL+ are expected to match but are untested.
8. **Long runs are untested.** The longest stream so far is 20 seconds (no errors).
9. **Streaming priority.** The loop runs at normal thread priority and does
   not yet ask Windows for pro-audio scheduling.
10. **A few details of the original driver are not established**: see the
    end of `spec/SPEC.md`.
