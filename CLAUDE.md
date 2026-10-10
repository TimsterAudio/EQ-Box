# EQ Box: working conventions

EQ Box is a native ten-band parametric EQ by Timster's Audio Works, written in Rust with egui (via eframe) for the panel. Read `docs/ARCHITECTURE.md` before you change code, and `docs/DESIGN.md` before you change the panel. The panel follows Drone Box's design language.

## Hard rules

- **Native, not HTML.** No web views or browser shells.
- **No network requests at runtime.** Fonts and the icon are built into the binary.
- **Keep the audio callbacks real-time safe.** They must not lock, allocate, do file I/O or log. The UI and the audio thread share state only through `Shared` (atomics) and the analyser ring buffer.
- **DSP stays free of UI and audio-API code.** `dsp.rs` and `apo.rs` are plain Rust and unit tested. Every backend runs audio through `audio::Processor`.
- **Never break saved settings.** `Saved` in `ui/app.rs` uses `#[serde(default)]`. Add new fields with defaults, and never rename or repurpose existing ones.
- **Keep the APO preset format compatible** with Equalizer APO, both ways.
- **Keep the panel ranges:** gain +12 / −24 dB, Q 0.1 to 20, 20 Hz to 20 kHz, preamp +12 / −24 dB.

## Checks before pushing

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo clippy --target x86_64-pc-windows-gnu   # Windows still compiles
```

## Workflow

- Work on a feature branch. `main` always holds a working version.
- Open a pull request when the user asks, or merge when they ask.
- For every user-visible change, add a line under **Unreleased** in `CHANGELOG.md`.
- For a release: move Unreleased into a section headed `## X.Y.Z – YYYY-MM-DD` (the release notes are taken from it), and bump `version` in `Cargo.toml` and the version in `README.md`. Merging that to `main` publishes the release: `.github/workflows/release.yml` creates the `vX.Y.Z` tag and release with the Linux and Windows packages. No manual tagging is needed.
