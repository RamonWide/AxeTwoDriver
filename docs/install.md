# Installing and first steps

AxeTwoDriver is experimental. It has been installed and used on one computer
with one Axe-Fx II XL. Read `ROADMAP.md` for what works.

You need Windows 10 or 11 (64-bit), an administrator account, and an Axe-Fx II
connected by USB and switched on. **Fractal Audio's own driver package must be
installed first**: the unit needs a small USB firmware at every power-on, and
AxeTwoDriver sends the copy that package put on your computer. It contains no
firmware of its own.

## Install

1. Download `AxeTwoDriver-<version>-windows-x64.zip` from the
   [latest release](https://github.com/RamonWide/AxeTwoDriver/releases/latest)
   and unzip it.
2. Close your audio programs.
3. Double-click **`axetwo-setup.exe`** and answer **Yes** when Windows asks.

The installer shows each step in a window and says so if something needs
attention. It:

1. copies the programs to `C:\Program Files\AxeTwoDriver`;
2. takes the original audio driver off the unit, after saving a copy in
   `C:\ProgramData\AxeTwoDriver\original-driver`;
3. puts Microsoft's WinUSB driver, which is part of Windows, on the unit's
   audio function. No driver file and no certificate is installed;
4. sets up firmware loading: a small background program, started with
   Windows, sends the unit its USB firmware whenever it is switched on. The
   original package's kernel loader is switched off, not removed;
5. registers the audio driver library for DAWs, adds the control panel to the
   Start menu and lists AxeTwoDriver under Installed apps.

It can be run again at any time, for example after connecting the unit to a
different USB port. A log is kept in `C:\ProgramData\AxeTwoDriver\install.log`.

## Use

| | |
|---|---|
| **MIDI** | The unit appears as the MIDI port "AXE-FX II Audio Intfc" in editors and other MIDI programs. |
| **Audio** | In your DAW, choose the audio device **AxeTwoDriver**: 4 inputs, 2 outputs, 48 kHz. 64-bit programs only, one at a time. |
| **Settings** | **AxeTwoDriver Control Panel** in the Start menu, or the control-panel button in your DAW's audio settings. |

The control panel has the USB streaming mode (how much playback is kept
queued for the unit, from 1 to 32 ms) and the buffer size suggested to audio
programs. A running program is asked to restart its audio engine when you
change them.

AxeTwoDriver does not add a Windows sound device, so browsers and media
players cannot play through the unit.

## Remove

**Settings > Apps > Installed apps > AxeTwoDriver > Uninstall.** This
unregisters the library, switches the original firmware loader back on, hands
the unit's audio back to Windows and removes the programs. The saved copy of the original audio driver stays, and the
uninstaller prints the command that puts it back.

## Testing from the command line

```powershell
& "C:\Program Files\AxeTwoDriver\axetwo.exe" status
& "C:\Program Files\AxeTwoDriver\axetwo.exe" capture 5 take.wav   # record five seconds, four channels
& "C:\Program Files\AxeTwoDriver\axetwo.exe" tone 3 -30           # play a quiet 440 Hz tone
```

`axetwo.exe` is a command-line tool: run it from a terminal. Double-clicking
it only flashes a window. Turn your monitors down before the tone test.

## Troubleshooting

| What you see | What it means |
|---|---|
| Installer: "no Axe-Fx II found" | Connect the unit, switch it on, run the installer again |
| Installer: "its USB firmware was not loaded" | Fractal Audio's driver package is not installed, so there is no firmware to send |
| The unit stays "waiting for its USB firmware" after power-on | Look at `C:\ProgramData\AxeTwoDriver\loader.log`; `axetwo firmware load` tries once by hand. Uninstalling AxeTwoDriver returns loading to the original loader |
| Installer: "could not be copied" | An audio program still has the library open; close it and run the installer again |
| DAW: AxeTwoDriver is listed but will not open | Another program is using the unit, or the unit is off |
| `opening the device ... is another program using it?` | The same, from the command-line tool |
