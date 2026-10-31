//! One-shot sound effects, rendered to PCM once at startup.

use super::synth::{Noise, Osc, SR, Wave, midi_hz, normalize, render};
use super::{songs, tracker, voice};

/// Every effect in the game.
pub struct Bank {
    pub accept: Vec<f32>,
    pub golden: Vec<f32>,
    /// A dive-save one lane down.
    pub save: Vec<f32>,
    pub jar_full: Vec<f32>,
    pub spit: Vec<f32>,
    pub step: Vec<f32>,
    pub tip: Vec<f32>,
    pub crumble: Vec<f32>,
    pub spider: Vec<f32>,
    pub bite: Vec<f32>,
    pub munch: Vec<f32>,
    pub popup: Vec<f32>,
    pub popup_ok: Vec<f32>,
    pub menu: Vec<f32>,
    pub nope: Vec<f32>,
    /// "D'oh!" followed by the sad trombone.
    pub doh: Vec<f32>,
    pub level_start: Vec<f32>,
    pub level_clear: Vec<f32>,
    pub game_over: Vec<f32>,
}

impl Bank {
    pub fn new() -> Self {
        Self {
            accept: arpeggio(&[72, 76, 79], 0.035, Wave::Pulse(0.25), 0.05),
            golden: arpeggio(
                &[84, 88, 91, 96, 91, 96, 100],
                0.035,
                Wave::Pulse(0.125),
                0.15,
            ),
            save: arpeggio(&[67, 74, 79, 83, 86], 0.03, Wave::Pulse(0.25), 0.1),
            jar_full: arpeggio(&[60, 64, 67, 72, 76, 79, 84], 0.045, Wave::Pulse(0.5), 0.25),
            spit: sweep(400.0, 900.0, 0.07, Wave::Pulse(0.5), 0.4),
            step: sweep(1200.0, 900.0, 0.025, Wave::Pulse(0.5), 0.25),
            tip: sweep(700.0, 350.0, 0.12, Wave::Triangle, 0.6),
            crumble: crumble(),
            spider: spider(),
            bite: bite(),
            munch: munch(),
            popup: arpeggio(&[79, 72, 79, 72], 0.06, Wave::Pulse(0.25), 0.05),
            popup_ok: arpeggio(&[67, 72, 76, 79, 84], 0.03, Wave::Pulse(0.25), 0.1),
            menu: sweep(900.0, 1100.0, 0.03, Wave::Pulse(0.25), 0.3),
            nope: sweep(220.0, 110.0, 0.18, Wave::Pulse(0.5), 0.5),
            doh: doh_with_trombone(),
            level_start: tracker::render_song(songs::level_start()),
            level_clear: tracker::render_song(songs::level_clear()),
            game_over: tracker::render_song(songs::game_over()),
        }
    }
}

impl Default for Bank {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick notes in a row, each decaying, with a tail.
fn arpeggio(notes: &[u8], step: f32, wave: Wave, tail: f32) -> Vec<f32> {
    let total = step * notes.len() as f32 + tail;
    let mut osc = Osc::default();
    let mut buf = render(total, |t| {
        let i = ((t / step) as usize).min(notes.len() - 1);
        let local = t - i as f32 * step;
        let env = (-local * 14.0).exp() * (1.0 - t / total).max(0.0);
        osc.sample(wave, midi_hz(notes[i])) * env
    });
    normalize(&mut buf, 0.6);
    buf
}

/// Exponential pitch sweep.
fn sweep(from: f32, to: f32, secs: f32, wave: Wave, peak: f32) -> Vec<f32> {
    let mut osc = Osc::default();
    let mut buf = render(secs, |t| {
        let k = t / secs;
        let hz = from * (to / from).powf(k);
        osc.sample(wave, hz) * (1.0 - k)
    });
    normalize(&mut buf, peak);
    buf
}

fn crumble() -> Vec<f32> {
    let mut n = Noise::default();
    let mut osc = Osc::default();
    let mut buf = render(0.35, |t| {
        // Crackly bursts: noise gated by a fast random-ish rhythm.
        let gate = if ((t * 60.0) as u32).is_multiple_of(3) {
            0.4
        } else {
            1.0
        };
        let thud = osc.sample(Wave::Triangle, 90.0 - 60.0 * t) * (-t * 25.0).exp();
        (n.sample(5000.0 + 3000.0 * (t * 40.0).sin()) * gate * (-t * 9.0).exp() + thud) * 0.8
    });
    normalize(&mut buf, 0.7);
    buf
}

fn spider() -> Vec<f32> {
    // Descending "zip" of silk, wobbly.
    let mut osc = Osc::default();
    let mut buf = render(0.6, |t| {
        let hz = 1400.0 * (1.0 - t * 1.2).max(0.15) + 60.0 * (t * 70.0).sin();
        osc.sample(Wave::Pulse(0.125), hz) * (1.0 - t / 0.6) * 0.5
    });
    normalize(&mut buf, 0.4);
    buf
}

fn bite() -> Vec<f32> {
    let mut n = Noise::default();
    let mut osc = Osc::default();
    let mut buf = render(0.3, |t| {
        let crunch = n.sample(14000.0) * (-t * 30.0).exp();
        let zap =
            osc.sample(Wave::Pulse(0.5), 1800.0 * (-t * 8.0).exp() + 100.0) * (-t * 6.0).exp();
        crunch * 0.6 + zap * 0.5
    });
    normalize(&mut buf, 0.8);
    buf
}

fn munch() -> Vec<f32> {
    let mut n = Noise::default();
    let mut buf = render(0.32, |t| {
        let chomp = ((t * 12.0).fract() < 0.5) as u8 as f32;
        n.sample(3000.0) * chomp * (1.0 - t / 0.32)
    });
    normalize(&mut buf, 0.5);
    buf
}

fn doh_with_trombone() -> Vec<f32> {
    let mut out = voice::doh(1.0);
    // A beat of silence, then the trombone.
    out.extend(std::iter::repeat_n(0.0, (SR * 0.12) as usize));
    let mut horn = tracker::render_song(songs::sad_trombone());
    normalize(&mut horn, 0.55);
    out.extend(horn);
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_effect_renders() {
        let b = super::Bank::new();
        for (name, pcm) in [
            ("accept", &b.accept),
            ("doh", &b.doh),
            ("crumble", &b.crumble),
            ("level_clear", &b.level_clear),
        ] {
            assert!(!pcm.is_empty(), "{name}");
            assert!(
                pcm.iter().all(|s| s.is_finite() && s.abs() <= 1.0),
                "{name}"
            );
        }
    }
}
