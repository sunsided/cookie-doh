//! All audio is generated in code: no sample files.
//!
//! - [`synth`]: chip-style oscillators, envelopes and drum voices.
//! - [`tracker`]: a tiny pattern player for the music, pulled on the audio thread.
//! - [`songs`]: the tunes, written in tracker notation.
//! - [`sfx`]: one-shot effects rendered to PCM at startup.
//! - [`voice`]: a formant synthesizer that says "D'oh!".

pub mod sfx;
pub mod songs;
pub mod synth;
pub mod tracker;
pub mod voice;

/// Sample rate for everything we synthesize; kira resamples to the device.
pub const SAMPLE_RATE: u32 = 32_000;
