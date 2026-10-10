//! Filter maths and the real-time equaliser.
//!
//! Everything here is free of UI and audio-API code so it can be unit tested
//! and shared by every backend.

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

pub const NUM_BANDS: usize = 10;
pub const GAIN_MIN_DB: f32 = -24.0;
pub const GAIN_MAX_DB: f32 = 12.0;
pub const Q_MIN: f32 = 0.1;
pub const Q_MAX: f32 = 20.0;
pub const FREQ_MIN: f32 = 20.0;
pub const FREQ_MAX: f32 = 20_000.0;
pub const PREAMP_MIN_DB: f32 = -24.0;
pub const PREAMP_MAX_DB: f32 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterKind {
    /// Peaking / bell.
    Peak,
    LowShelf,
    HighShelf,
}

impl FilterKind {
    pub const ALL: [FilterKind; 3] = [
        FilterKind::LowShelf,
        FilterKind::Peak,
        FilterKind::HighShelf,
    ];

    pub fn short(self) -> &'static str {
        match self {
            FilterKind::Peak => "BELL",
            FilterKind::LowShelf => "LO SH",
            FilterKind::HighShelf => "HI SH",
        }
    }

    pub fn to_index(self) -> u32 {
        match self {
            FilterKind::Peak => 0,
            FilterKind::LowShelf => 1,
            FilterKind::HighShelf => 2,
        }
    }

    pub fn from_index(i: u32) -> Self {
        match i {
            1 => FilterKind::LowShelf,
            2 => FilterKind::HighShelf,
            _ => FilterKind::Peak,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BandParams {
    pub enabled: bool,
    pub kind: FilterKind,
    pub freq: f32,
    pub gain_db: f32,
    pub q: f32,
}

impl BandParams {
    /// Keep every field inside the ranges the panel allows.
    pub fn clamped(mut self) -> Self {
        self.freq = clamp_finite(self.freq, FREQ_MIN, FREQ_MAX, 1000.0);
        self.gain_db = clamp_finite(self.gain_db, GAIN_MIN_DB, GAIN_MAX_DB, 0.0);
        self.q = clamp_finite(self.q, Q_MIN, Q_MAX, 0.707);
        self
    }

    /// The gain the filter actually applies: a switched-off band is flat.
    pub fn effective_gain(&self) -> f32 {
        if self.enabled {
            self.gain_db
        } else {
            0.0
        }
    }
}

fn clamp_finite(v: f32, lo: f32, hi: f32, fallback: f32) -> f32 {
    if v.is_finite() {
        v.clamp(lo, hi)
    } else {
        fallback
    }
}

/// The starting layout: octave-spaced bands, shelves on the outside.
pub fn default_bands() -> [BandParams; NUM_BANDS] {
    const FREQS: [f32; NUM_BANDS] = [
        31.0, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
    ];
    std::array::from_fn(|i| BandParams {
        enabled: true,
        kind: match i {
            0 => FilterKind::LowShelf,
            9 => FilterKind::HighShelf,
            _ => FilterKind::Peak,
        },
        freq: FREQS[i],
        gain_db: 0.0,
        q: if i == 0 || i == 9 { 0.707 } else { 1.0 },
    })
}

/// Normalised biquad coefficients (a0 == 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coeffs {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

impl Coeffs {
    pub const IDENTITY: Coeffs = Coeffs {
        b0: 1.0,
        b1: 0.0,
        b2: 0.0,
        a1: 0.0,
        a2: 0.0,
    };

    /// RBJ "Audio EQ Cookbook" designs. Shelves use the Q form, which is what
    /// APO-format presets' `LSC`/`HSC` filters use, so imported presets match.
    pub fn design(kind: FilterKind, freq: f64, gain_db: f64, q: f64, sample_rate: f64) -> Coeffs {
        if gain_db.abs() < 1e-6 || sample_rate <= 0.0 {
            return Coeffs::IDENTITY;
        }
        let freq = freq.clamp(1.0, sample_rate * 0.499);
        let q = q.max(1e-3);
        let a = 10f64.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq / sample_rate;
        let (sin_w0, cos_w0) = w0.sin_cos();
        let alpha = sin_w0 / (2.0 * q);

        let (b0, b1, b2, a0, a1, a2) = match kind {
            FilterKind::Peak => (
                1.0 + alpha * a,
                -2.0 * cos_w0,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos_w0,
                1.0 - alpha / a,
            ),
            FilterKind::LowShelf => {
                let sq = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 + sq),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0),
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 - sq),
                    (a + 1.0) + (a - 1.0) * cos_w0 + sq,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0),
                    (a + 1.0) + (a - 1.0) * cos_w0 - sq,
                )
            }
            FilterKind::HighShelf => {
                let sq = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 + sq),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0),
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 - sq),
                    (a + 1.0) - (a - 1.0) * cos_w0 + sq,
                    2.0 * ((a - 1.0) - (a + 1.0) * cos_w0),
                    (a + 1.0) - (a - 1.0) * cos_w0 - sq,
                )
            }
        };
        Coeffs {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    pub fn for_band(band: &BandParams, sample_rate: f64) -> Coeffs {
        Coeffs::design(
            band.kind,
            band.freq as f64,
            band.effective_gain() as f64,
            band.q as f64,
            sample_rate,
        )
    }

    /// Magnitude response in dB at `freq`.
    pub fn magnitude_db(&self, freq: f64, sample_rate: f64) -> f64 {
        let w = 2.0 * PI * freq / sample_rate;
        let (s1, c1) = w.sin_cos();
        let (s2, c2) = (2.0 * w).sin_cos();
        // H(z) evaluated at z = e^{jw}
        let nr = self.b0 + self.b1 * c1 + self.b2 * c2;
        let ni = -(self.b1 * s1 + self.b2 * s2);
        let dr = 1.0 + self.a1 * c1 + self.a2 * c2;
        let di = -(self.a1 * s1 + self.a2 * s2);
        let num = nr * nr + ni * ni;
        let den = (dr * dr + di * di).max(1e-30);
        10.0 * (num / den).max(1e-30).log10()
    }
}

/// Response of a whole band set (plus preamp) at one frequency, in dB.
#[cfg_attr(not(test), allow(dead_code))]
pub fn total_response_db(bands: &[BandParams], preamp_db: f32, freq: f64, sample_rate: f64) -> f64 {
    bands
        .iter()
        .map(|b| Coeffs::for_band(b, sample_rate).magnitude_db(freq, sample_rate))
        .sum::<f64>()
        + preamp_db as f64
}

/// Transposed direct form II, one channel.
#[derive(Clone, Copy, Default)]
struct BiquadState {
    z1: f64,
    z2: f64,
}

impl BiquadState {
    #[inline]
    fn tick(&mut self, c: &Coeffs, x: f64) -> f64 {
        let y = c.b0 * x + self.z1;
        self.z1 = c.b1 * x - c.a1 * y + self.z2;
        self.z2 = c.b2 * x - c.a2 * y;
        y
    }
}

/// Parameters smoothed at block rate so knob moves never click.
#[derive(Clone, Copy)]
struct SmoothedBand {
    kind: FilterKind,
    log_freq: f64,
    gain_db: f64,
    log_q: f64,
}

impl SmoothedBand {
    fn from_params(p: &BandParams) -> Self {
        SmoothedBand {
            kind: p.kind,
            log_freq: (p.freq as f64).ln(),
            gain_db: p.effective_gain() as f64,
            log_q: (p.q as f64).ln(),
        }
    }

    /// Move towards the target; returns true while still moving.
    fn approach(&mut self, target: &SmoothedBand, k: f64) -> bool {
        self.kind = target.kind;
        let mut moving = false;
        for (cur, tgt, eps) in [
            (&mut self.log_freq, target.log_freq, 1e-5),
            (&mut self.gain_db, target.gain_db, 1e-4),
            (&mut self.log_q, target.log_q, 1e-5),
        ] {
            let d = tgt - *cur;
            if d.abs() > eps {
                *cur += d * k;
                moving = true;
            } else {
                *cur = tgt;
            }
        }
        moving
    }

    fn coeffs(&self, sample_rate: f64) -> Coeffs {
        Coeffs::design(
            self.kind,
            self.log_freq.exp(),
            self.gain_db,
            self.log_q.exp(),
            sample_rate,
        )
    }
}

/// A snapshot of everything the audio thread needs, read once per block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EqSettings {
    pub bands: [BandParams; NUM_BANDS],
    pub preamp_db: f32,
    pub bypass: bool,
}

impl Default for EqSettings {
    fn default() -> Self {
        EqSettings {
            bands: default_bands(),
            preamp_db: 0.0,
            bypass: false,
        }
    }
}

const BLOCK: usize = 32;

/// The real-time equaliser: interleaved f32 in, interleaved f32 out.
pub struct Equalizer {
    sample_rate: f64,
    channels: usize,
    current: [SmoothedBand; NUM_BANDS],
    coeffs: [Coeffs; NUM_BANDS],
    state: Vec<[BiquadState; NUM_BANDS]>,
    preamp_gain: f64,
    wet: f64,
    smooth_k: f64,
    gain_k: f64,
    primed: bool,
}

impl Equalizer {
    pub fn new(sample_rate: f64, channels: usize) -> Self {
        // ~30 ms time constant for parameter glides, evaluated per block.
        let blocks_per_sec = sample_rate / BLOCK as f64;
        let smooth_k = 1.0 - (-1.0 / (0.03 * blocks_per_sec)).exp();
        let gain_k = 1.0 - (-1.0 / (0.01 * sample_rate)).exp();
        let defaults = EqSettings::default();
        Equalizer {
            sample_rate,
            channels: channels.max(1),
            current: defaults.bands.map(|b| SmoothedBand::from_params(&b)),
            coeffs: [Coeffs::IDENTITY; NUM_BANDS],
            state: vec![[BiquadState::default(); NUM_BANDS]; channels.max(1)],
            preamp_gain: 1.0,
            wet: 1.0,
            smooth_k,
            gain_k,
            primed: false,
        }
    }

    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Process interleaved audio in place.
    pub fn process(&mut self, buf: &mut [f32], settings: &EqSettings) {
        let targets = settings
            .bands
            .map(|b| SmoothedBand::from_params(&b.clamped()));
        if !self.primed {
            // Start exactly on the stored settings, with no glide from flat.
            self.current = targets;
            for i in 0..NUM_BANDS {
                self.coeffs[i] = self.current[i].coeffs(self.sample_rate);
            }
            self.preamp_gain = db_to_gain(settings.preamp_db as f64);
            self.wet = if settings.bypass { 0.0 } else { 1.0 };
            self.primed = true;
        }
        let target_gain = db_to_gain(settings.preamp_db as f64);
        let target_wet = if settings.bypass { 0.0 } else { 1.0 };
        let ch = self.channels;

        for block in buf.chunks_mut(BLOCK * ch) {
            for i in 0..NUM_BANDS {
                let kind_changed = self.current[i].kind != targets[i].kind;
                if self.current[i].approach(&targets[i], self.smooth_k) || kind_changed {
                    self.coeffs[i] = self.current[i].coeffs(self.sample_rate);
                }
            }
            for frame in block.chunks_mut(ch) {
                self.preamp_gain += (target_gain - self.preamp_gain) * self.gain_k;
                self.wet += (target_wet - self.wet) * self.gain_k;
                for (c, s) in frame.iter_mut().enumerate() {
                    let dry = *s as f64;
                    let mut y = dry * self.preamp_gain;
                    let st = &mut self.state[c];
                    for i in 0..NUM_BANDS {
                        y = st[i].tick(&self.coeffs[i], y);
                    }
                    let out = dry + (y - dry) * self.wet;
                    *s = if out.is_finite() { out as f32 } else { 0.0 };
                }
            }
        }
        // Guard against denormals and blow-ups after silence or a bad buffer.
        for st in &mut self.state {
            for b in st.iter_mut() {
                if !b.z1.is_finite() || !b.z2.is_finite() {
                    *b = BiquadState::default();
                }
                if b.z1.abs() < 1e-25 {
                    b.z1 = 0.0;
                }
                if b.z2.abs() < 1e-25 {
                    b.z2 = 0.0;
                }
            }
        }
    }
}

pub fn db_to_gain(db: f64) -> f64 {
    10f64.powf(db / 20.0)
}

pub fn gain_to_db(g: f32) -> f32 {
    if g <= 1e-6 {
        -120.0
    } else {
        20.0 * g.log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FS: f64 = 48_000.0;

    #[test]
    fn peak_hits_its_gain_at_centre() {
        for g in [-24.0, -6.0, 3.0, 12.0] {
            let c = Coeffs::design(FilterKind::Peak, 1000.0, g, 1.0, FS);
            assert!((c.magnitude_db(1000.0, FS) - g).abs() < 0.01, "gain {g}");
            assert!(c.magnitude_db(20.0, FS).abs() < 0.2);
        }
    }

    #[test]
    fn shelves_reach_full_gain_on_their_side() {
        let ls = Coeffs::design(FilterKind::LowShelf, 200.0, 6.0, 0.707, FS);
        assert!((ls.magnitude_db(20.0, FS) - 6.0).abs() < 0.1);
        assert!(ls.magnitude_db(10_000.0, FS).abs() < 0.1);
        assert!((ls.magnitude_db(200.0, FS) - 3.0).abs() < 0.1);
        let hs = Coeffs::design(FilterKind::HighShelf, 5000.0, -12.0, 0.707, FS);
        assert!((hs.magnitude_db(20_000.0, FS) + 12.0).abs() < 0.3);
        assert!(hs.magnitude_db(100.0, FS).abs() < 0.1);
    }

    #[test]
    fn disabled_band_is_flat() {
        let mut b = default_bands()[4];
        b.gain_db = 9.0;
        b.enabled = false;
        assert_eq!(Coeffs::for_band(&b, FS), Coeffs::IDENTITY);
    }

    #[test]
    fn clamps_to_panel_ranges() {
        let b = BandParams {
            enabled: true,
            kind: FilterKind::Peak,
            freq: 5.0,
            gain_db: 40.0,
            q: 0.0,
        }
        .clamped();
        assert_eq!((b.freq, b.gain_db, b.q), (FREQ_MIN, GAIN_MAX_DB, Q_MIN));
        let b = BandParams {
            gain_db: -99.0,
            freq: f32::NAN,
            ..b
        }
        .clamped();
        assert_eq!((b.freq, b.gain_db), (1000.0, GAIN_MIN_DB));
    }

    /// Feed a sine through the processor and compare with the analytic curve.
    #[test]
    fn processor_matches_design_response() {
        let mut s = EqSettings::default();
        s.bands[5].gain_db = 9.0;
        s.bands[0].gain_db = -6.0;
        s.preamp_db = -3.0;
        let mut eq = Equalizer::new(FS, 2);
        for &f in &[50.0, 1000.0, 3000.0] {
            let n = 48_000;
            let mut buf: Vec<f32> = (0..n)
                .flat_map(|i| {
                    let v = (2.0 * PI * f * i as f64 / FS).sin() as f32 * 0.1;
                    [v, v]
                })
                .collect();
            eq.process(&mut buf, &s);
            let tail = &buf[buf.len() / 2..];
            let rms_out =
                (tail.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / tail.len() as f64).sqrt();
            let rms_in = 0.1 / 2f64.sqrt();
            let measured = 20.0 * (rms_out / rms_in).log10();
            let expected = total_response_db(&s.bands, s.preamp_db, f, FS);
            assert!(
                (measured - expected).abs() < 0.1,
                "{f} Hz: {measured} vs {expected}"
            );
        }
    }

    #[test]
    fn bypass_passes_audio_untouched() {
        let mut s = EqSettings::default();
        s.bands[3].gain_db = 12.0;
        s.preamp_db = -10.0;
        s.bypass = true;
        let mut eq = Equalizer::new(FS, 1);
        let input: Vec<f32> = (0..4096)
            .map(|i| ((i * 7919) % 200) as f32 / 200.0 - 0.5)
            .collect();
        let mut buf = input.clone();
        eq.process(&mut buf, &s);
        for (a, b) in input.iter().zip(&buf) {
            assert!((a - b).abs() < 1e-6);
        }
    }
}
