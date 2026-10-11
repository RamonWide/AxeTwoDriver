AxeTwoDriver
============

An open-source USB driver for the Axe-Fx II on Windows 10 and 11.

TO INSTALL

  1. Connect the Axe-Fx II by USB and switch it on.
  2. Close your audio programs.
  3. Double-click axetwo-setup.exe and answer Yes when Windows asks.

  The installer shows each step and tells you if something needs attention.
  Fractal Audio's own driver package must be installed first: AxeTwoDriver
  still relies on its firmware loader. The installer takes the original
  audio driver off the unit (keeping a backup) and leaves the loader alone.

AFTERWARDS

  MIDI     The unit appears as the MIDI port "AXE-FX II Audio Intfc".
  Audio    In your DAW, choose the audio device "AxeTwoDriver"
           (4 inputs, 2 outputs, 48 kHz). 64-bit programs only.
  Settings "AxeTwoDriver Control Panel" in the Start menu, or the control
           panel button in your DAW's audio settings.

  AxeTwoDriver does not add a Windows sound device, so browsers and media
  players cannot play through the unit.

TO REMOVE

  Settings > Apps > Installed apps > AxeTwoDriver > Uninstall.

WHAT IS IN THIS FOLDER

  axetwo-setup.exe   the installer                          MIT license
  axetwo.exe         command-line tool for testing          MIT license
  axetwo-panel.exe   the control panel                      MIT license
  axetwo-loader.exe  loads the unit's USB firmware at       MIT license
                     power-on (runs in the background)
  axetwo_daw.dll     audio driver library for DAWs          GPL version 3

  The MIT license is in LICENSE.txt, the GPL in LICENSE-DAW.txt, and the
  licenses of the open-source libraries the programs are built from in
  THIRD-PARTY-LICENSES.txt.
  Source code (SOURCE.txt names the exact commits this download was built from):
    https://github.com/RamonWide/AxeTwoDriver
    https://github.com/RamonWide/AxeTwoDriver-DAW

AxeTwoDriver is an independent project, not affiliated with or endorsed by
Fractal Audio Systems or Steinberg. Axe-Fx is a trademark of Fractal Audio
Systems, LLC. ASIO is a trademark and software of Steinberg Media
Technologies GmbH. There is no warranty: you use it with your own equipment
at your own risk.
