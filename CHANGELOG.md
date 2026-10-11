# Changelog

## Unreleased

- Windows: a system-wide engine (test build). EQ Box's EQ runs inside Windows' own audio engine and equalises every app on the outputs you choose, with no virtual cable. Install it with `install-engine.ps1`, then choose **Windows (system-wide)** under System.
- WASAPI no longer offers "Loopback:" inputs, and never captures an output device, because looping an output back into the EQ can feed back.
- The preset format is now called the APO preset format in dialogs and docs.
- Windows engine: install it from EQ Box (Install / Add / Remove under System) or by double-clicking the `.cmd` files, with no PowerShell typing. The display shows whether the engine is installed and running.
- Windows engine: an output's own driver effect (speaker tuning, "enhancements") is kept and runs after EQ Box's EQ, instead of having to be switched off.
- Windows engine: the event log only gets start-up, stream starts and problems.
- New filter types: high-pass (HPF), low-pass (LPF) and notch, on a second row of type keys. They have a fixed shape (12 dB per octave for HPF and LPF), so a band's gain doesn't apply to them, and they fade in and out when switched. Presets with `HP`, `HPQ`, `LP`, `LPQ` and `NO` filters now load, and EQ Box saves them in that format.
- Each band's On key lights up in the band's colour, instead of having an LED.
- Analyser colours: the **Colour** tab on the Response frame picks red, amber, green, ice, white, rainbow (bass to treble) or heat (by level), and the **Style** tab picks fill and line, line only, or fill only.
- **EQ Box Analyser**, a separate program: the spectrum analyser on its own, listening to what an output plays or to an input. Curve, bars or LED views at 1/3, 1/6 or 1/12 octave; the same colour schemes; adjustable rise, fall and peak hold; range and slope; and for any screen, a resizable window with hideable controls, full screen and always-on-top.

## 0.1.0 – 2026-10-10

First version.

- Ten bands, each a bell, low shelf or high shelf, with gain from +12 to −24 dB, a frequency from 20 Hz to 20 kHz, and a Q from 0.1 to 20.
- Preamp, bypass, Flat and Reset.
- Loads and saves APO-format EQ presets (such as AutoEQ's `ParametricEQ.txt`). You can use the Load key, drop a file on the window, or give a path on the command line.
- Response graph with per-band curves, the total response, a live spectrum analyser and draggable band points.
- Audio on Linux: a native PipeWire sink called "EQ Box", plus PipeWire and ALSA device modes.
- Audio on Windows: WASAPI (with loopback capture), and ASIO with the `asio` build feature.
- Drone Box panel: black or silver, colour or pointer knobs, LED display, level meters and red readouts.
- Remembers its settings between runs.
- Downloads for Linux, Windows and Windows + ASIO on the GitHub Releases page.
