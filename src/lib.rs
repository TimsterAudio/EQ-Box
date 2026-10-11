//! The code behind EQ Box's two programs: `eq-box`, the equaliser, and
//! `eq-box-analyser`, the stand-alone spectrum analyser.

// Index loops read best in the per-sample DSP code.
#![allow(clippy::needless_range_loop)]

pub mod analyzer;
pub mod audio;
pub mod icon;
pub mod ui;

pub use eq_core::{dsp, preset, shared};
