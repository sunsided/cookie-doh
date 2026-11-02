//! One-shot sound effects, rendered to PCM once at startup on the toolkit's synth (synthie).

use game_toolkit::prelude::Synth;
use game_toolkit::prelude::synth::{AudioEvent, DrumHit, MidiNote, SynthParams};
use game_toolkit::prelude::synthie::prelude::{
    LfoParams, LfoShape, LfoTarget, ModEnvParams, Waveform,
};

use super::songs::{adsr, chip};
use super::{SAMPLE_RATE, normalize, songs, voice};

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
    /// The Cache Cleaner sets off: "boop-beep, boop-beep".
    pub cleaner: Vec<f32>,
    pub bonk: Vec<f32>,
    pub slurp: Vec<f32>,
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
        let mut s = Fx::new();
        Self {
            accept: s.arpeggio(&[72, 76, 79], 0.035, 0.25, 0.05),
            golden: s.arpeggio(&[84, 88, 91, 96, 91, 96, 100], 0.035, 0.125, 0.15),
            save: s.arpeggio(&[67, 74, 79, 83, 86], 0.03, 0.25, 0.1),
            jar_full: s.arpeggio(&[60, 64, 67, 72, 76, 79, 84], 0.045, 0.5, 0.25),
            spit: s.sweep(400.0, 900.0, 0.07, Waveform::Pulse, 0.4),
            step: s.sweep(1200.0, 900.0, 0.025, Waveform::Pulse, 0.25),
            tip: s.sweep(700.0, 350.0, 0.12, Waveform::Triangle, 0.6),
            crumble: s.crumble(),
            spider: s.spider(),
            bite: s.bite(),
            munch: s.munch(),
            cleaner: s.cleaner(),
            bonk: s.bonk(),
            slurp: s.slurp(),
            popup: s.arpeggio(&[79, 72, 79, 72], 0.06, 0.25, 0.05),
            popup_ok: s.arpeggio(&[67, 72, 76, 79, 84], 0.03, 0.25, 0.1),
            menu: s.sweep(900.0, 1100.0, 0.03, Waveform::Pulse, 0.3),
            nope: s.sweep(220.0, 110.0, 0.18, Waveform::Pulse, 0.5),
            doh: doh_with_trombone(),
            level_start: jingle(songs::level_start()),
            level_clear: jingle(songs::level_clear()),
            game_over: jingle(songs::game_over()),
        }
    }
}

impl Default for Bank {
    fn default() -> Self {
        Self::new()
    }
}

/// `(time, event)` pairs for [`Synth::render_phrase`].
type Phrase = Vec<(f32, AudioEvent)>;

fn on(t: f32, note: u8) -> (f32, AudioEvent) {
    (t, AudioEvent::NoteOn(MidiNote(note)))
}

fn off(t: f32, note: u8) -> (f32, AudioEvent) {
    (t, AudioEvent::NoteOff(MidiNote(note)))
}

/// `note` for `len` seconds starting at `t`.
fn note(phrase: &mut Phrase, t: f32, note: u8, len: f32) {
    phrase.push(on(t, note));
    phrase.push(off(t + len, note));
}

/// The MIDI note nearest to `hz`, and the cents it is off by (for the oscillator's detune).
fn note_of(hz: f32) -> (u8, f32) {
    let n = 69.0 + 12.0 * (hz / 440.0).log2();
    let r = n.round().clamp(0.0, 127.0);
    (r as u8, (n - r) * 100.0)
}

/// A percussive blip: full on, gone after `decay` seconds.
fn blip(wave: Waveform, pulse_width: f32, decay: f32) -> SynthParams {
    chip(wave, pulse_width, 0.7, adsr(0.001, decay, 0.0, 0.02))
}

/// The noise oscillator (an LFSR clocked at the note's frequency) as a patch.
fn noise(env: game_toolkit::prelude::synthie::prelude::EnvParams) -> SynthParams {
    chip(Waveform::Noise, 0.5, 0.7, env)
}

/// Fade `buf` out linearly to silence at its end, then bring it back up to `peak`.
fn fade_out(buf: &mut [f32], peak: f32) {
    let n = buf.len().max(1) as f32;
    for (i, s) in buf.iter_mut().enumerate() {
        *s *= 1.0 - i as f32 / n;
    }
    normalize(buf, peak);
}

/// Sum several renders (unnormalized) sample by sample.
fn mix(parts: &[Vec<f32>]) -> Vec<f32> {
    let len = parts.iter().map(Vec::len).max().unwrap_or(0);
    let mut out = vec![0.0; len];
    for p in parts {
        for (o, s) in out.iter_mut().zip(p) {
            *o += s;
        }
    }
    out
}

/// The effect builders, sharing one synth.
struct Fx {
    synth: Synth,
}

impl Fx {
    fn new() -> Self {
        Self {
            synth: Synth::new(SAMPLE_RATE),
        }
    }

    /// Render `phrase` on `patch` with `tail` seconds after the last event, at `peak`.
    fn phrase(
        &mut self,
        phrase: &[(f32, AudioEvent)],
        patch: SynthParams,
        tail: f32,
        peak: f32,
    ) -> Vec<f32> {
        self.synth.set_target_peak(peak);
        self.synth.render_phrase(phrase, patch, tail)
    }

    /// Quick pulse notes in a row, each a decaying blip.
    fn arpeggio(&mut self, notes: &[u8], step: f32, pulse_width: f32, tail: f32) -> Vec<f32> {
        let mut p = Phrase::new();
        for (i, &n) in notes.iter().enumerate() {
            note(&mut p, i as f32 * step, n, step);
        }
        let patch = blip(Waveform::Pulse, pulse_width, 0.2);
        let mut buf = self.phrase(&p, patch, tail, 0.6);
        fade_out(&mut buf, 0.6);
        buf
    }

    /// An exponential pitch sweep from `from` to `to` Hz (at most an octave either way),
    /// fading out over `secs`.
    fn sweep(&mut self, from: f32, to: f32, secs: f32, wave: Waveform, peak: f32) -> Vec<f32> {
        let (n, cents) = note_of(from);
        let mut patch = chip(wave, 0.5, 0.7, adsr(0.001, secs, 0.0, 0.01));
        patch.osc.detune = cents;
        // The pitch envelope bends by `depth` octaves as it rises: linear in octaves is
        // exponential in Hz.
        patch.pitch_env = ModEnvParams {
            attack: secs,
            decay: 0.0,
            sustain: 1.0,
            release: 0.0,
            depth: (to / from).log2().clamp(-1.0, 1.0),
        };
        self.synth.set_target_peak(peak);
        self.synth.render(patch, n, secs, 0.01)
    }

    /// Crackly bursts of noise over a dull thud.
    fn crumble(&mut self) -> Vec<f32> {
        let mut patch = noise(adsr(0.001, 0.3, 0.0, 0.05));
        // Chopped at 20 Hz, and the noise clock jumping around for crackle.
        patch.lfo = LfoParams {
            lfo_rate: 20.0,
            lfo_depth: 1.0,
            lfo_target: LfoTarget::Volume,
            lfo_shape: LfoShape::Square,
        };
        patch.lfo2 = LfoParams {
            lfo_rate: 40.0,
            lfo_depth: 1.0,
            lfo_target: LfoTarget::Pitch,
            lfo_shape: LfoShape::SampleHold,
        };
        let p = vec![
            (0.0, AudioEvent::Drum(DrumHit::TomLow)),
            on(0.0, note_of(5000.0).0),
            off(0.3, note_of(5000.0).0),
        ];
        self.phrase(&p, patch, 0.05, 0.7)
    }

    /// A descending, wobbly zip of silk.
    fn spider(&mut self) -> Vec<f32> {
        let mut patch = blip(Waveform::Pulse, 0.125, 1.0);
        patch.env = adsr(0.001, 0.1, 1.0, 0.01);
        patch.lfo = LfoParams {
            lfo_rate: 11.0,
            lfo_depth: 0.4,
            lfo_target: LfoTarget::Pitch,
            ..LfoParams::default()
        };
        // Chromatic steps from ~1400 Hz down to ~220 Hz: a chip "zip".
        let (top, _) = note_of(1400.0);
        let (bottom, _) = note_of(220.0);
        let step = 0.5 / f32::from(top - bottom);
        let mut p = Phrase::new();
        for (i, n) in (bottom..=top).rev().enumerate() {
            note(&mut p, i as f32 * step, n, step);
        }
        let mut buf = self.phrase(&p, patch, 0.08, 0.4);
        fade_out(&mut buf, 0.4);
        buf
    }

    /// A crunch and a falling zap.
    fn bite(&mut self) -> Vec<f32> {
        self.synth.set_target_peak(0.0);
        let crunch = self.synth.render_phrase(
            &[
                (0.0, AudioEvent::Drum(DrumHit::Clap)),
                on(0.0, 127),
                off(0.08, 127),
            ],
            noise(adsr(0.001, 0.08, 0.0, 0.01)),
            0.05,
        );
        let mut zap = self.sweep(1800.0, 900.0, 0.3, Waveform::Pulse, 0.0);
        for s in &mut zap {
            *s *= 0.8;
        }
        let mut buf = mix(&[crunch, zap]);
        normalize(&mut buf, 0.8);
        buf
    }

    /// Four chomps of noise.
    fn munch(&mut self) -> Vec<f32> {
        let (n, _) = note_of(3000.0);
        let mut p = Phrase::new();
        for k in 0..4 {
            note(&mut p, k as f32 / 12.0, n, 1.0 / 24.0);
        }
        let mut buf = self.phrase(&p, noise(adsr(0.002, 0.04, 0.6, 0.01)), 0.03, 0.5);
        fade_out(&mut buf, 0.5);
        buf
    }

    /// "Boop-beep, boop-beep, boop-beep."
    fn cleaner(&mut self) -> Vec<f32> {
        let mut p = Phrase::new();
        for k in 0..6 {
            let n = if k % 2 == 0 { 81 } else { 88 };
            note(&mut p, k as f32 * 0.12, n, 0.09);
        }
        let patch = chip(Waveform::Pulse, 0.25, 0.7, adsr(0.001, 0.01, 1.0, 0.005));
        self.phrase(&p, patch, 0.03, 0.45)
    }

    /// Head meets wall.
    fn bonk(&mut self) -> Vec<f32> {
        self.synth.set_target_peak(0.0);
        let tone = self.sweep(300.0, 150.0, 0.25, Waveform::Triangle, 0.0);
        let hit = self.synth.render_phrase(
            &[
                (0.0, AudioEvent::Drum(DrumHit::TomMid)),
                on(0.0, note_of(6000.0).0),
                off(0.02, note_of(6000.0).0),
            ],
            noise(adsr(0.001, 0.02, 0.0, 0.01)),
            0.25,
        );
        let mut buf = mix(&[tone, hit]);
        normalize(&mut buf, 0.85);
        buf
    }

    /// Noise clocked ever higher: the sound of something going up a tube.
    fn slurp(&mut self) -> Vec<f32> {
        let (lo, _) = note_of(1500.0);
        let steps = 14;
        let step = 0.35 / steps as f32;
        let mut p = Phrase::new();
        for k in 0..steps {
            let n = lo + ((127 - lo) as usize * k / (steps - 1)) as u8;
            note(&mut p, k as f32 * step, n, step);
        }
        let mut buf = self.phrase(&p, noise(adsr(0.001, 0.01, 1.0, 0.005)), 0.01, 0.45);
        fade_out(&mut buf, 0.45);
        buf
    }
}

/// A one-shot tune rendered with its tail, as loud as the effects.
fn jingle(tune: super::music::Tune) -> Vec<f32> {
    let mut buf = tune.render_once(SAMPLE_RATE, 0.6);
    normalize(&mut buf, 0.85);
    buf
}

fn doh_with_trombone() -> Vec<f32> {
    let mut out = voice::doh(1.0);
    // A beat of silence, then the trombone.
    out.extend(std::iter::repeat_n(
        0.0,
        (SAMPLE_RATE as f32 * 0.12) as usize,
    ));
    let mut horn = songs::sad_trombone().render_once(SAMPLE_RATE, 0.5);
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
            ("golden", &b.golden),
            ("save", &b.save),
            ("jar_full", &b.jar_full),
            ("spit", &b.spit),
            ("step", &b.step),
            ("tip", &b.tip),
            ("crumble", &b.crumble),
            ("spider", &b.spider),
            ("bite", &b.bite),
            ("munch", &b.munch),
            ("cleaner", &b.cleaner),
            ("bonk", &b.bonk),
            ("slurp", &b.slurp),
            ("popup", &b.popup),
            ("popup_ok", &b.popup_ok),
            ("menu", &b.menu),
            ("nope", &b.nope),
            ("doh", &b.doh),
            ("level_start", &b.level_start),
            ("level_clear", &b.level_clear),
            ("game_over", &b.game_over),
        ] {
            assert!(
                pcm.iter().all(|s| s.is_finite() && s.abs() <= 1.0),
                "{name}"
            );
            let peak = pcm.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak > 0.1, "{name} is silent (peak {peak})");
        }
    }
}
