# Architecture

```
            ┌──────────── UI thread (eframe / egui) ────────────┐
            │ ui/app.rs   panel layout, presets, audio control  │
            │ ui/graph.rs response graph + band points          │
            │ ui/widgets.rs knobs, faders, keys, LED display    │
            └──────┬───────────────────────────────▲────────────┘
     Shared::store │ (atomics)                     │ analyser ring (rtrb)
                   ▼                               │ meters (atomics)
            ┌──────────── audio thread(s) ─────────┴────────────┐
            │ audio::Processor → dsp::Equalizer                  │
            │   PipeWire sink   (audio/pipewire_sink.rs)         │
            │   cpal devices    (audio/cpal_backend.rs)          │
            │     WASAPI · ASIO · ALSA · PipeWire devices        │
            └────────────────────────────────────────────────────┘
```

## Modules

The code is a Cargo workspace:

- **`crates/eq-core`**: everything with no UI or audio-API code. Both the app and the Windows engine are built on it.
- **the app** (the root package, `src/`)
- **`crates/eq-box-apo`**: the Windows system-wide engine DLL.

| File | What it does |
|---|---|
| `eq-core/src/dsp.rs` | Band parameters and ranges, and the RBJ-cookbook biquad designs (bell, and Q-form shelves to match the APO preset format's `LSC`/`HSC`). It also has the magnitude-response maths for the graph, and `Equalizer`, the real-time processor. |
| `eq-core/src/preset.rs` | Parses and writes APO-format EQ presets. |
| `eq-core/src/shared.rs` | Lock-free parameters (UI to audio), peak meters (audio to UI) and the analyser tap. |
| `eq-core/src/engine_settings.rs` | The settings file (`%ProgramData%\EQ Box\engine.json`) the app writes and the Windows engine reads. Writes are atomic: a temporary file, then a rename. |
| `eq-box-apo/src/apo.rs` | The Audio Processing Object: the COM class factory, format checks (32-bit float, in place), `LockForProcess` and the real-time `APOProcess`, which runs the same `Equalizer`. |
| `eq-box-apo/src/settings_watch.rs` | One thread per audio-engine process. It watches the settings file and stores changes into `Shared` for every engine instance. |
| `eq-box-apo/examples/apo_harness.rs` | Loads the DLL through COM, as the Windows audio engine does, and checks the EQ on a test tone. CI runs it on Windows, and it also runs under Wine. |
| `packaging/windows/*.ps1` | Test-build setup: install the engine on one output (with a backup), and remove it again. |
| `analyzer.rs` | An 8192-point FFT with a Hann window, fast attack and slow release, read at any frequency span. |
| `audio/mod.rs` | The backend list, device lists, `start()` and `Processor`, which meters, equalises and feeds the analyser. |
| `audio/pipewire_sink.rs` | Two PipeWire streams in one link group: an `Audio/Sink` node called "EQ Box", and a playback stream to the chosen output. They're joined by a ring buffer, the same shape as `pw-loopback`. |
| `audio/windows_engine.rs` | "Windows (system-wide)": writes the panel to the engine's settings file when it changes, and drives the meters and analyser from WASAPI loopback. It never touches the audio path. |
| `audio/cpal_backend.rs` | Any cpal host. An input stream and an output stream are joined by a ring buffer with a set latency and drift correction. On WASAPI, `loopback:` inputs capture an output device. |
| `ui/theme.rs` | Drone Box's colours and fonts, the Black and Silver panels, and the knob styles. |

## Real-time processing

- `Equalizer::process` reads one snapshot of the settings for each buffer.
- Frequency (as a log), gain and Q (as a log) glide to their targets with a time constant of about 30 ms, and the coefficients are worked out again every 32 samples while anything is still moving. Moving a control never clicks.
- The preamp and the bypass crossfade are smoothed per sample.
- A band that's switched off glides to 0 dB, which is identity for every filter type, so switching a band on or off is click-free too.
- The filter state is kept in `f64`, in transposed direct form II, and is flushed of denormals and non-finite values after each buffer.

## Threads

- **PipeWire:** one thread runs the PipeWire main loop. The stream `process` callbacks run on PipeWire's real-time data thread (`RT_PROCESS`). Stopping sends a message over a `pipewire::channel` that quits the loop.
- **cpal:** a dedicated thread owns the streams, because some hosts' streams can't move between threads. Dropping `Running` stops that thread.

## Tests

`cargo test` checks:

- the filter designs against their analytic gain at known points
- that a sine through the real processor matches the designed curve to within 0.1 dB
- that bypass is bit-transparent
- the APO parser: AutoEQ files, slopes, `BW Oct`, comments, the 10-band limit, range limiting, and round trips
