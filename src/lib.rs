//! The code behind EQ Box's two programs: `eq-box`, the equaliser, and
//! `sp-10`, the SP-10 Spectrum Analyser.

// Index loops read best in the per-sample DSP code.
#![allow(clippy::needless_range_loop)]

pub mod analyzer;
pub mod audio;
pub mod icon;
pub mod ui;

pub use eq_core::{dsp, preset, shared};
