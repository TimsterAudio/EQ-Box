//! Real-time spectrum of the EQ's output, for drawing behind the curve.

use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

const N: usize = 8192;

/// How the display moves: rise and fall times, and how long peaks hold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ballistics {
    /// Time constant going up, in ms (0 = instant).
    pub rise_ms: f32,
    /// Time constant going down, in ms.
    pub fall_ms: f32,
    /// How long a peak marker holds before falling, in seconds (0 = off).
    pub hold_s: f32,
    /// How fast held peaks fall afterwards, in dB per second.
    pub peak_fall_db_s: f32,
}

impl Default for Ballistics {
    /// EQ Box's own analyser: instant rise, a fifth of a second to fall.
    fn default() -> Self {
        Ballistics {
            rise_ms: 0.0,
            fall_ms: 215.0,
            hold_s: 0.0,
            peak_fall_db_s: 20.0,
        }
    }
}

pub struct Analyzer {
    rx: Option<rtrb::Consumer<f32>>,
    history: Vec<f32>,
    write: usize,
    since_fft: usize,
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    work: Vec<Complex<f32>>,
    /// Smoothed level per bin, in dB.
    pub bins_db: Vec<f32>,
    /// Held peak per bin, in dB (when `ballistics.hold_s` > 0).
    pub peaks_db: Vec<f32>,
    peak_age: Vec<f32>,
    pub active: bool,
    pub ballistics: Ballistics,
    /// Samples between spectra: smaller is smoother motion, more work.
    pub hop: usize,
    pub sample_rate: f32,
}

impl Default for Analyzer {
    fn default() -> Self {
        let window = (0..N)
            .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / N as f32).cos())
            .collect();
        Analyzer {
            rx: None,
            history: vec![0.0; N],
            write: 0,
            since_fft: 0,
            fft: FftPlanner::new().plan_fft_forward(N),
            window,
            work: vec![Complex::default(); N],
            bins_db: vec![-140.0; N / 2],
            peaks_db: vec![-140.0; N / 2],
            peak_age: vec![0.0; N / 2],
            active: false,
            ballistics: Ballistics::default(),
            hop: 2048,
            sample_rate: 48_000.0,
        }
    }
}

impl Analyzer {
    pub fn connect(&mut self, rx: rtrb::Consumer<f32>) {
        self.rx = Some(rx);
        self.bins_db.fill(-140.0);
        self.peaks_db.fill(-140.0);
    }

    /// Pull new audio; returns true if the spectrum changed.
    pub fn update(&mut self) -> bool {
        let Some(rx) = self.rx.as_mut() else {
            self.active = false;
            return false;
        };
        let n = rx.slots();
        if n == 0 {
            self.active = false;
            return false;
        }
        self.active = true;
        if let Ok(chunk) = rx.read_chunk(n) {
            let (a, b) = chunk.as_slices();
            for &s in a.iter().chain(b) {
                self.history[self.write] = s;
                self.write = (self.write + 1) % N;
            }
            chunk.commit_all();
        }
        self.since_fft += n;
        if self.since_fft < self.hop.clamp(256, N) {
            return false;
        }
        self.since_fft = 0;
        for i in 0..N {
            let s = self.history[(self.write + i) % N];
            self.work[i] = Complex::new(s * self.window[i], 0.0);
        }
        self.fft.process(&mut self.work);
        // A full-scale sine reads 0 dB.
        let norm = 4.0 / N as f32;
        let b = self.ballistics;
        let dt = self.hop.clamp(256, N) as f32 / self.sample_rate.max(8000.0);
        let k = |ms: f32| {
            if ms <= 0.0 {
                1.0
            } else {
                1.0 - (-dt * 1000.0 / ms).exp()
            }
        };
        let (k_rise, k_fall) = (k(b.rise_ms), k(b.fall_ms));
        for (i, slot) in self.bins_db.iter_mut().enumerate() {
            let db = 20.0 * (self.work[i].norm() * norm).max(1e-7).log10();
            let k = if db > *slot { k_rise } else { k_fall };
            *slot += (db - *slot) * k;
        }
        if b.hold_s > 0.0 {
            for i in 0..self.bins_db.len() {
                if self.bins_db[i] >= self.peaks_db[i] {
                    self.peaks_db[i] = self.bins_db[i];
                    self.peak_age[i] = 0.0;
                } else {
                    self.peak_age[i] += dt;
                    if self.peak_age[i] > b.hold_s {
                        self.peaks_db[i] =
                            (self.peaks_db[i] - b.peak_fall_db_s * dt).max(self.bins_db[i]);
                    }
                }
            }
        }
        true
    }

    /// Level at a frequency band [f0, f1] in Hz: the loudest bin, or an
    /// interpolated value where bins are sparser than pixels.
    pub fn level(&self, f0: f32, f1: f32, sample_rate: f32) -> f32 {
        level_in(&self.bins_db, f0, f1, sample_rate)
    }

    /// Like [`Analyzer::level`], for the held peaks.
    pub fn peak_level(&self, f0: f32, f1: f32, sample_rate: f32) -> f32 {
        level_in(&self.peaks_db, f0, f1, sample_rate)
    }
}

fn level_in(bins_db: &[f32], f0: f32, f1: f32, sample_rate: f32) -> f32 {
    let bin_hz = sample_rate / N as f32;
    let b0 = f0 / bin_hz;
    let b1 = f1 / bin_hz;
    let last = bins_db.len() - 1;
    if b1 - b0 < 1.0 {
        let c = ((b0 + b1) * 0.5).clamp(0.0, last as f32);
        let i = c.floor() as usize;
        let j = (i + 1).min(last);
        let t = c - i as f32;
        bins_db[i] * (1.0 - t) + bins_db[j] * t
    } else {
        let i0 = (b0.ceil() as usize).min(last);
        let i1 = (b1.floor() as usize).min(last);
        bins_db[i0..=i1.max(i0)]
            .iter()
            .cloned()
            .fold(-140.0, f32::max)
    }
}
