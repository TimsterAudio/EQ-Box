# Changelog

## Unreleased

- Windows: a system-wide engine (test build). EQ Box's EQ runs inside Windows' own audio engine and equalises every app on the outputs you choose, with no virtual cable. Install it with `install-engine.ps1`, then choose **Windows (system-wide)** under System.
- WASAPI no longer offers "Loopback:" inputs, and never captures an output device, because looping an output back into the EQ can feed back.
- The preset format is now called the APO preset format in dialogs and docs.

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
