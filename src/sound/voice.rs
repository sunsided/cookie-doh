//! A small formant synthesizer, just good enough to exclaim "D'oh!".
//!
//! A glottal pulse train runs through a cascade of three two-pole resonators (Klatt style)
//! whose frequencies glide from the alveolar "d" locus into an "oh" that rounds off towards
//! "oo". The result is crushed to 6 bits at ~8 kHz, which is what makes it sound like a
//! 1980s speech chip instead of a person.

use super::synth::{Noise, SR, render};

/// A Klatt-style resonator: unity gain at DC, peak at `f`, bandwidth `bw`.
#[derive(Default)]
struct Resonator {
    y1: f32,
    y2: f32,
}

impl Resonator {
    fn sample(&mut self, x: f32, f: f32, bw: f32) -> f32 {
        let t = 1.0 / SR;
        let c = -(-2.0 * std::f32::consts::PI * bw * t).exp();
        let b = 2.0
            * (-std::f32::consts::PI * bw * t).exp()
            * (2.0 * std::f32::consts::PI * f * t).cos();
        let a = 1.0 - b - c;
        let y = a * x + b * self.y1 + c * self.y2;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

fn smooth(t: f32, t0: f32, t1: f32) -> f32 {
    let k = ((t - t0) / (t1 - t0)).clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

fn lerp(a: f32, b: f32, k: f32) -> f32 {
    a + (b - a) * k
}

/// Synthesize "D'oh!" at [`super::SAMPLE_RATE`]. `pitch` scales the voice (1.0 = default).
pub fn doh(pitch: f32) -> Vec<f32> {
    const LEN: f32 = 0.62;
    let mut f1 = Resonator::default();
    let mut f2 = Resonator::default();
    let mut f3 = Resonator::default();
    let mut noise = Noise::default();
    let mut phase = 0.0f32;
    let mut lp = 0.0f32;

    let mut out = render(LEN, |t| {
        // Pitch: an exclamation jumps up, then sags as the breath runs out.
        let f0 = pitch
            * if t < 0.07 {
                lerp(150.0, 190.0, smooth(t, 0.0, 0.07))
            } else {
                lerp(190.0, 105.0, smooth(t, 0.07, LEN))
            }
            * (1.0 + 0.012 * (t * 31.0).sin());
        phase += f0 / SR;
        if phase >= 1.0 {
            phase -= 1.0;
        }
        // Glottal pulse: a rising ramp with a sharp closure, lowpassed a little.
        let glottal = if phase < 0.6 {
            (phase / 0.6 * std::f32::consts::PI).sin().powi(2)
        } else {
            0.0
        };
        let src = glottal - 0.35;
        lp += (src - lp) * 0.5;

        // Formant trajectory: "d" locus (F2 high) -> "aw" -> rounded "oh/oo".
        let k_d = smooth(t, 0.02, 0.09);
        let k_o = smooth(t, 0.18, LEN);
        let fr1 = lerp(lerp(300.0, 640.0, k_d), 390.0, k_o);
        let fr2 = lerp(lerp(1700.0, 1000.0, k_d), 760.0, k_o);
        let fr3 = lerp(2600.0, 2400.0, k_d);

        // Voicing: a short closure murmur, the release burst, then the vowel.
        let amp = if t < 0.018 {
            0.12
        } else {
            smooth(t, 0.018, 0.04) * (1.0 - smooth(t, LEN - 0.16, LEN))
        };
        let burst = if (0.016..0.026).contains(&t) {
            noise.sample(12000.0) * 0.4
        } else {
            0.0
        };
        let x = lp * amp + burst;
        let y = f1.sample(x, fr1, 90.0);
        let y = f2.sample(y, fr2, 110.0);
        f3.sample(y, fr3, 160.0)
    });

    super::synth::normalize(&mut out, 0.95);
    crush(&mut out, 8000.0, 6);
    out
}

/// Sample-and-hold to `rate` Hz and quantize to `bits`.
fn crush(buf: &mut [f32], rate: f32, bits: u32) {
    let hold = (SR / rate).max(1.0);
    let levels = (1 << (bits - 1)) as f32;
    let mut acc = 0.0;
    let mut held = 0.0;
    for s in buf {
        if acc <= 0.0 {
            held = (*s * levels).round() / levels;
            acc += hold;
        }
        acc -= 1.0;
        *s = held;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn doh_is_audible_and_bounded() {
        let pcm = super::doh(1.0);
        assert!(pcm.len() > 10_000);
        assert!(pcm.iter().all(|s| s.abs() <= 1.0));
        let rms = (pcm.iter().map(|s| s * s).sum::<f32>() / pcm.len() as f32).sqrt();
        assert!(rms > 0.05, "rms {rms}");
    }
}
