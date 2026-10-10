//! Real-time spectrum of the EQ's output, for drawing behind the curve.

use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

const N: usize = 8192;
const HOP: usize = 2048;

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
    pub active: bool,
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
            active: false,
        }
    }
}

impl Analyzer {
    pub fn connect(&mut self, rx: rtrb::Consumer<f32>) {
        self.rx = Some(rx);
        self.bins_db.fill(-140.0);
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
        if self.since_fft < HOP {
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
        for (i, slot) in self.bins_db.iter_mut().enumerate() {
            let db = 20.0 * (self.work[i].norm() * norm).max(1e-7).log10();
            // Fast attack, slow release.
            *slot = if db > *slot {
                db
            } else {
                *slot * 0.82 + db * 0.18
            };
        }
        true
    }

    /// Level at a frequency band [f0, f1] in Hz: the loudest bin, or an
    /// interpolated value where bins are sparser than pixels.
    pub fn level(&self, f0: f32, f1: f32, sample_rate: f32) -> f32 {
        let bin_hz = sample_rate / N as f32;
        let b0 = f0 / bin_hz;
        let b1 = f1 / bin_hz;
        let last = self.bins_db.len() - 1;
        if b1 - b0 < 1.0 {
            let c = ((b0 + b1) * 0.5).clamp(0.0, last as f32);
            let i = c.floor() as usize;
            let j = (i + 1).min(last);
            let t = c - i as f32;
            self.bins_db[i] * (1.0 - t) + self.bins_db[j] * t
        } else {
            let i0 = (b0.ceil() as usize).min(last);
            let i1 = (b1.floor() as usize).min(last);
            self.bins_db[i0..=i1.max(i0)]
                .iter()
                .cloned()
                .fold(-140.0, f32::max)
        }
    }
}
