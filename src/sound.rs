//! All audio is generated in code: no sample files.
//!
//! - [`music`]: songs on the toolkit's synth (synthie), streamed live or rendered offline.
//! - [`songs`]: the tunes, in tracker notation, and their chip patches.
//! - [`sfx`]: one-shot effects rendered to PCM at startup.
//! - [`voice`]: a formant synthesizer that says "D'oh!".

pub mod music;
pub mod sfx;
pub mod songs;
pub mod voice;

/// Sample rate for everything we render ahead of time; kira resamples to the device.
pub const SAMPLE_RATE: u32 = 44_100;

/// Scale `buf` so its loudest sample is `peak`.
pub fn normalize(buf: &mut [f32], peak: f32) {
    let m = buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if m > 1e-5 {
        for s in buf {
            *s *= peak / m;
        }
    }
}
