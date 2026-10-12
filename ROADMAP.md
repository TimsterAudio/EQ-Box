# Roadmap

Ideas and planned work for EQ Box (EQ-10) and SP-10. Tick items off (`[x]`) when they ship, and note the version.

## Easy enough for anyone

The aim: someone who has never heard of an equaliser can install and use EQ-10 and SP-10 on their own. On Windows that means download, double-click, **Yes**, check the speakers are ticked, **OK**, with nothing to type and no technical windows. In rough order of value:

- [ ] **A setup window instead of PowerShell.** A small setup helper (`eq-box-setup.exe`, in Rust, doing what the scripts do today) runs with the one Windows permission prompt, and EQ Box shows its own panel-styled window:
  - "Which speakers should EQ Box work on?", listing only connected outputs by their friendly names, with the one playing now already ticked.
  - A **Play test sound** button by each, so nobody has to know which name is which.
  - Plain progress with ticks ("Installing… ✓  Restarting Windows sound… ✓"), then "Done. Play some music and try the knobs." and an **OK** that closes it.
  - The same window for Remove and Update.
- [ ] **A proper installer.** `EQ-Box-Setup.exe` instead of a zip (no "extract it first" step), built by the release workflow. It installs EQ-10 and SP-10 to Program Files with Start menu shortcuts, appears in Settings › Apps for uninstalling, and can include the speaker page above so the engine is set up in the same pass. (Step 4 of [docs/WINDOWS_APO_PLAN.md](docs/WINDOWS_APO_PLAN.md).)
- [ ] **Code signing.** Sign the installer and programs so Windows doesn't show "Windows protected your PC". Needs a certificate or a signing service (a cost decision); until then, the download page shows the "More info › Run anyway" step with a picture.
- [ ] **Sensible first run.** When the engine is installed, EQ-10 starts in system-wide mode, running, with Auto preamp on, so there's no System setting or Run key to find. SP-10 starts on the default output.
- [ ] **One-click Update and Repair.** When EQ-10 is newer than the engine, **Update** with no questions. When a Windows update resets a device's effects and the engine stops being used, EQ-10 notices and offers **Repair**.
- [ ] **Fix it for me.** When "Audio enhancements" are off for the output, a **Turn on** button instead of directions to the setting.
- [ ] **A simple panel.** EQ-3 (Low / Mid / High) as a Bands option, with a few named starting points such as "Clearer speech", "More bass" and "Softer treble".
- [ ] **Help from a distance.** A **Copy report** button that copies what `diagnose-engine.ps1` shows today, to paste into a message.
- [ ] **Linux: no trip to sound settings.** A **Make EQ Box my output** button (sets the default sink), a **Start with my computer** option, and a single-file download (AppImage or Flatpak) so there's nothing to unpack.

## In progress

- [x] **System-wide EQ on Windows, with no virtual cable.** EQ Box's own engine plugs into the Windows audio engine as an Audio Processing Object (APO), the same way other EQ software does it. See [docs/WINDOWS_APO_PLAN.md](docs/WINDOWS_APO_PLAN.md). *(0.2.0, as a test build installed from EQ Box. Still to come: a full Windows installer.)*

## Planned

- [x] **Analyser colour options:** choose the colours of the spectrum analyser (fill, line, and scheme). *(0.2.0)*
- [x] **More filter types:** notch, high-pass (HPF) and low-pass (LPF), alongside bell and the shelves. Presets that use `NO`, `HP`/`HPQ` and `LP`/`LPQ` would then load instead of being skipped. *(0.2.0)*
- [x] **Band On/Off key without an LED:** drop the LED above each band's On key, and make the key itself light up when the band is on. *(0.2.0)*
- [x] **A separate analyser app:** just the spectrum analyser, as its own program, with options for different displays (sizes, layouts and screens), colours, and rise/decay (attack and release) times. *(0.2.0: SP-10 Spectrum Analyser.)*
