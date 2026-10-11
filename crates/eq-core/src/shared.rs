//! State shared between the UI and the audio thread, without locks.

use crate::dsp::{BandParams, EqSettings, FilterKind, NUM_BANDS};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed};

#[derive(Default)]
struct AtomicF32(AtomicU32);

impl AtomicF32 {
    fn load(&self) -> f32 {
        f32::from_bits(self.0.load(Relaxed))
    }
    fn store(&self, v: f32) {
        self.0.store(v.to_bits(), Relaxed)
    }
}

#[derive(Default)]
struct AtomicBand {
    enabled: AtomicBool,
    kind: AtomicU32,
    freq: AtomicF32,
    gain: AtomicF32,
    q: AtomicF32,
}

/// Parameters written by the UI and read by the audio thread once per buffer,
/// plus meters written by the audio thread for the UI.
#[derive(Default)]
pub struct Shared {
    bands: [AtomicBand; NUM_BANDS],
    preamp: AtomicF32,
    bypass: AtomicBool,
    /// Peak levels since the UI last read them: [in L, in R, out L, out R].
    peaks: [AtomicF32; 4],
    pub sample_rate: AtomicU32,
    pub dropouts: AtomicU64,
}

impl Shared {
    pub fn new(settings: &EqSettings) -> Self {
        let s = Shared::default();
        s.store(settings);
        s
    }

    pub fn store(&self, settings: &EqSettings) {
        for (a, b) in self.bands.iter().zip(&settings.bands) {
            a.enabled.store(b.enabled, Relaxed);
            a.kind.store(b.kind.to_index(), Relaxed);
            a.freq.store(b.freq);
            a.gain.store(b.gain_db);
            a.q.store(b.q);
        }
        self.preamp.store(settings.preamp_db);
        self.bypass.store(settings.bypass, Relaxed);
    }

    pub fn load(&self) -> EqSettings {
        EqSettings {
            bands: std::array::from_fn(|i| {
                let a = &self.bands[i];
                BandParams {
                    enabled: a.enabled.load(Relaxed),
                    kind: FilterKind::from_index(a.kind.load(Relaxed)),
                    freq: a.freq.load(),
                    gain_db: a.gain.load(),
                    q: a.q.load(),
                }
            }),
            preamp_db: self.preamp.load(),
            bypass: self.bypass.load(Relaxed),
        }
    }

    /// Record the peaks of an interleaved buffer (first two channels).
    pub fn meter(&self, buf: &[f32], channels: usize, output: bool) {
        let base = if output { 2 } else { 0 };
        let ch = channels.max(1);
        let mut pk = [0f32; 2];
        for frame in buf.chunks(ch) {
            pk[0] = pk[0].max(frame[0].abs());
            pk[1] = pk[1].max(frame.get(1).unwrap_or(&frame[0]).abs());
        }
        for (i, p) in pk.iter().enumerate() {
            let slot = &self.peaks[base + i];
            if *p > slot.load() {
                slot.store(*p);
            }
        }
    }

    /// Read and reset the peaks.
    pub fn take_peaks(&self) -> [f32; 4] {
        std::array::from_fn(|i| {
            let v = self.peaks[i].load();
            self.peaks[i].store(0.0);
            v
        })
    }
}

/// The audio side of the analyser feed: a mono mix of the output.
pub struct AnalyzerTap {
    tx: rtrb::Producer<f32>,
}

impl AnalyzerTap {
    pub fn push(&mut self, buf: &[f32], channels: usize) {
        let ch = channels.max(1);
        let scale = 1.0 / ch as f32;
        for frame in buf.chunks(ch) {
            if self.tx.push(frame.iter().sum::<f32>() * scale).is_err() {
                break; // UI isn't reading (minimised); drop the rest
            }
        }
    }
}

pub fn analyzer_channel() -> (AnalyzerTap, rtrb::Consumer<f32>) {
    let (tx, rx) = rtrb::RingBuffer::new(1 << 15);
    (AnalyzerTap { tx }, rx)
}
