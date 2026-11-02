//! The soundtrack: tracker lanes (see synthie's pattern notation) on chip-style patches.
//!
//! Every song keeps its melodic lanes on channels 0..=2 (3 for the cracktro's bass) with one
//! patch each, and its drum lane on a channel of its own or shared: drum hits play on the
//! synth's drum machine, not on the channel's voices.

use game_toolkit::prelude::synth::{ChannelNo, Song, SynthParams, Tempo};
use game_toolkit::prelude::synthie::prelude::{
    ArpMode, EnvParams, FilterParams, FxParams, GlobalParams, LfoParams, LfoTarget, OscParams,
    Waveform,
};

use super::music::Tune;

/// A raw chip voice: no glide, no reverb, the filter wide open. `sustain` is the level the
/// envelope settles at; synthie applies its envelope twice (before and after the filter),
/// so the audible level is its square.
pub fn chip(wave: Waveform, pulse_width: f32, volume: f32, env: EnvParams) -> SynthParams {
    SynthParams {
        osc: OscParams {
            waveform: wave,
            pulse_width,
            ..OscParams::default()
        },
        env,
        filter: FilterParams {
            cutoff: 14_000.0,
            resonance: 0.0,
            ..FilterParams::default()
        },
        fx: FxParams {
            reverb_mix: 0.0,
            ..FxParams::default()
        },
        global: GlobalParams {
            volume,
            glide_time: 0.0,
        },
        ..SynthParams::default()
    }
}

/// Attack, decay and release in seconds, sustain as a level.
pub const fn adsr(attack: f32, decay: f32, sustain: f32, release: f32) -> EnvParams {
    EnvParams {
        attack,
        decay,
        sustain,
        release,
        env_reverse: false,
    }
}

/// Pitch vibrato: `semitones` deep at `rate` Hz.
fn vibrato(mut p: SynthParams, semitones: f32, rate: f32) -> SynthParams {
    // The LFO bends by `depth` tenths of an octave.
    p.lfo = LfoParams {
        lfo_rate: rate,
        lfo_depth: semitones / 1.2,
        lfo_target: LfoTarget::Pitch,
        ..LfoParams::default()
    };
    p
}

/// A light hall on the whole mix: synthie's reverb is shared, taken from channel 0's patch.
fn with_reverb(mut p: SynthParams, mix: f32) -> SynthParams {
    p.fx.reverb_mix = mix;
    p.fx.reverb_size = 0.6;
    p
}

fn lead() -> SynthParams {
    vibrato(
        chip(Waveform::Pulse, 0.25, 0.32, adsr(0.004, 0.25, 0.74, 0.15)),
        0.25,
        6.0,
    )
}

/// The triangle bass, crushed to 4 bits like the NES triangle channel's 16 steps.
fn bass() -> SynthParams {
    let mut p = chip(Waveform::Triangle, 0.5, 0.55, adsr(0.004, 0.4, 0.9, 0.1));
    p.crusher.bits = 4.0;
    p
}

/// Thin pulse chords, played as fast chip arpeggios (`C4^047`) at 50 notes a second.
fn chords() -> SynthParams {
    let mut p = chip(Waveform::Pulse, 0.125, 0.16, adsr(0.002, 0.1, 0.55, 0.1));
    p.arp.rate = 50.0;
    p.arp.gate = 1.0;
    p.arp.mode = ArpMode::Up;
    p
}

fn tune(rpm: f32, lanes: &[(u8, &str)], patches: Vec<SynthParams>) -> Tune {
    let song = lanes.iter().fold(
        Song::new(Tempo::rows_per_minute(rpm)),
        |song, &(ch, lane)| {
            song.lane(ChannelNo(ch), lane)
                .unwrap_or_else(|e| panic!("bad lane: {e}"))
        },
    );
    Tune::new(song, patches)
}

/// In-game: bouncy C major, eighth-note rows at 150 bpm.
pub fn game() -> Tune {
    tune(
        300.0,
        &[
            (
                0,
                "E5 . G5 E5 C5 . D5 E5 | C5 . A4 . C5 E5 D5 C5 |
                 A4 . C5 A4 F4 . G4 A4 | B4 . D5 . G5 . - . |
                 E5 G5 C6 . B5 G5 E5 . | A5 . G5 E5 C5 . E5 . |
                 F5 E5 D5 C5 A4 . C5 . | D5 . B4 G4 D5 E5 F5 D5 |",
            ),
            (
                1,
                "- C4^047 - C4^047 - C4^047 - C4^047 | - A3^037 - A3^037 - A3^037 - A3^037 |
                 - F3^047 - F3^047 - F3^047 - F3^047 | - G3^047 - G3^047 - G3^047 - G3^047 |
                 - C4^047 - C4^047 - C4^047 - C4^047 | - A3^037 - A3^037 - A3^037 - A3^037 |
                 - F3^047 - F3^047 - F3^047 - F3^047 | - G3^047 - G3^047 - G3^04a - G3^04a |",
            ),
            (
                2,
                "C2 C3 C2 C3 C2 C3 G1 B1 | A1 A2 A1 A2 A1 A2 E2 G2 |
                 F1 F2 F1 F2 F1 F2 C2 E2 | G1 G2 G1 G2 G1 G2 D2 B1 |
                 C2 C3 C2 C3 C2 C3 G1 B1 | A1 A2 A1 A2 A1 A2 E2 G2 |
                 F1 F2 F1 F2 F1 F2 C2 E2 | G1 G2 G1 G2 D2 D2 B1 G1 |",
            ),
            (
                3,
                "k h s h k k s h | k h s h k k s h | k h s h k k s h | k h s h k s s s |
                 k h s h k k s h | k h s h k k s h | k h s h k k s h | k h s h s s s s |",
            ),
        ],
        vec![lead(), chords(), bass()],
    )
}

/// Title screen: a smug little ii-V-I lounge loop.
pub fn title() -> Tune {
    let lead = with_reverb(vibrato(lead(), 0.3, 5.0), 0.12);
    tune(
        232.0,
        &[
            (
                0,
                "F5 . E5 D5 . A4 C5 . | B4 . D5 F5 . E5 D5 . |
                 E5 . . G5 . . E5 C5 | D5 . . . - . . . |
                 A5 . G5 F5 . D5 F5 . | G5 . F5 D5 . B4 D5 . |
                 C5 . E5 G5 . C#5 E5 G5 | F5 . E5 . D5 . B4 - |",
            ),
            (
                1,
                "D4^37a . . . D4^37a . . - | G3^47a . . . G3^47a . . - |
                 C4^47b . . . C4^47b . . - | C4^47b . . . C4^47b . . - |
                 D4^37a . . . D4^37a . . - | G3^47a . . . G3^47a . . - |
                 C4^47b . . . A3^47a . . - | D4^37a . . . G3^47a . . - |",
            ),
            (
                2,
                "D2 . F2 . A2 . C3 . | G1 . B1 . D2 . F2 . |
                 C2 . E2 . G2 . B2 . | C2 . G1 . A1 . B1 . |
                 D2 . F2 . A2 . C3 . | G1 . B1 . D2 . F2 . |
                 C2 . E2 . A1 . C#2 . | D2 . A1 . G1 . B1 . |",
            ),
            (
                3,
                "k . h . s . h h | k . h . s . h h | k . h . s . h h | k . h . s . h s |
                 k . h . s . h h | k . h . s . h h | k . h . s . h h | k . s . s s s s |",
            ),
        ],
        vec![lead, chords(), bass()],
    )
}

/// "Get ready!"
pub fn level_start() -> Tune {
    tune(
        600.0,
        &[
            (0, "G4 C5 E5 G5 . . E5 G5 . . . -"),
            (1, "C2 . . . . . C3 . . . . -"),
            (2, "k . . . s . k . s . . ."),
        ],
        vec![lead(), bass()],
    )
}

/// Every cookie accepted.
pub fn level_clear() -> Tune {
    tune(
        480.0,
        &[
            (0, "C5 E5 G5 C6 . . G5 C6 . . E6 . D6 . C6 . . . . -"),
            (
                1,
                "C4^047 . . . . . . . . . F4^047 . G4^047 . C4^047 . . . . -",
            ),
            (2, "C2 . . . G2 . . . C3 . F2 . G2 . C2 . . . . -"),
            (3, "k . . . s . k . s . k . s . k . . . . ."),
        ],
        vec![lead(), chords(), bass()],
    )
}

/// Session expired.
pub fn game_over() -> Tune {
    let lead = vibrato(
        chip(Waveform::Pulse, 0.5, 0.3, adsr(0.004, 0.25, 0.84, 0.15)),
        0.4,
        5.0,
    );
    tune(
        200.0,
        &[
            (0, "C5 . G4 . Eb4 . D4 . C4 . . . . -"),
            (1, "C3 . . . G2 . . . C2 . . . . -"),
            (2, "k . . . k . . . s . . . . ."),
        ],
        vec![lead, bass()],
    )
}

/// The sad trombone after a "D'oh!".
pub fn sad_trombone() -> Tune {
    let horn = vibrato(
        chip(Waveform::Sawtooth, 0.5, 0.35, adsr(0.03, 0.5, 0.9, 0.4)),
        0.45,
        6.5,
    );
    tune(320.0, &[(0, "G3 . F#3 . F3 . E3 . . . -")], vec![horn])
}

/// `pattern` delayed by `rows`, wrapping around: the classic C64 trick of a second, quieter
/// channel playing the lead a few rows late as an echo.
fn delayed(pattern: &str, rows: usize) -> String {
    let mut t: Vec<&str> = pattern.split_whitespace().filter(|t| *t != "|").collect();
    let n = rows.min(t.len());
    t.rotate_right(n);
    t.join(" ")
}

/// The cracktro: wistful A minor, no hi-hats, nothing that grates on loop number ten.
pub fn cracktro() -> Tune {
    let lead = "A4 . C5 . E5 . D5 C5 | C5 . A4 . F4 . G4 A4 |
                G4 . E4 . G4 . C5 . | B4 . A4 . G4 . - . |
                E5 . A5 . G5 . E5 D5 | C5 . F5 . E5 . C5 A4 |
                G4 . C5 . E5 . G5 . | F5 . E5 . D5 . B4 - |";
    let voice = vibrato(
        chip(Waveform::Pulse, 0.5, 0.2, adsr(0.01, 0.33, 0.7, 0.5)),
        0.2,
        5.0,
    );
    let echo = SynthParams {
        global: GlobalParams {
            volume: 0.07,
            ..voice.global.clone()
        },
        ..voice.clone()
    };
    let mut pad = chords();
    pad.global.volume = 0.09;
    pad.env = adsr(0.004, 0.5, 0.7, 0.75);
    let mut soft_bass = bass();
    soft_bass.global.volume = 0.45;
    soft_bass.env.release = 0.3;
    tune(
        200.0,
        &[
            (0, lead),
            (1, &delayed(lead, 3)),
            (
                2,
                "A3^037 . . . . . . . | F3^047 . . . . . . . | C4^047 . . . . . . . | G3^047 . . . . . . . |
                 A3^037 . . . . . . . | F3^047 . . . . . . . | C4^047 . . . . . . . | G3^047 . . . . . . . |",
            ),
            (
                3,
                "A1 . A2 . A1 . E2 . | F1 . F2 . F1 . C2 . | C2 . C3 . C2 . G1 . | G1 . G2 . G1 . D2 . |
                 A1 . A2 . A1 . E2 . | F1 . F2 . F1 . C2 . | C2 . C3 . C2 . G1 . | G1 . G2 . G1 . D2 . |",
            ),
            // Drums don't take a channel's voices, so they can share the lead's.
            (
                0,
                "k . . . s . . . | k . . . s . . . | k . . . s . . . | k . . k s . . . |
                 k . . . s . . . | k . . . s . . . | k . . . s . . . | k . . k s . s . |",
            ),
        ],
        vec![with_reverb(voice, 0.15), echo, pad, soft_bass],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_is_the_lead_rotated() {
        assert_eq!(delayed("A4 . | B4 C5", 1), "C5 A4 . B4");
    }

    #[test]
    fn all_songs_have_equal_lane_lengths() {
        for tune in [game(), title(), cracktro()] {
            let lens: Vec<usize> = tune.song.lanes.iter().map(|l| l.rows.len()).collect();
            assert!(lens.windows(2).all(|w| w[0] == w[1]), "{lens:?}");
        }
    }

    #[test]
    fn every_melodic_lane_has_a_patch() {
        let all = [
            game(),
            title(),
            cracktro(),
            level_start(),
            level_clear(),
            game_over(),
            sad_trombone(),
        ];
        for tune in all {
            for lane in &tune.song.lanes {
                let drums_only = lane.rows.iter().all(|c| {
                    use game_toolkit::prelude::synthie::pattern::Action;
                    matches!(c.action, Action::Drums(_) | Action::Hold)
                });
                assert!(
                    drums_only || usize::from(lane.channel.0) < tune.patches.len(),
                    "lane on channel {} has no patch",
                    lane.channel.0
                );
            }
        }
    }

    /// A full loop of each song stays below the synth's clamp, and is clearly audible.
    #[test]
    fn music_never_clips() {
        for (name, tune) in [
            ("game", game()),
            ("title", title()),
            ("cracktro", cracktro()),
        ] {
            let secs = tune.song.duration_secs();
            let pcm = tune.render(super::super::SAMPLE_RATE, secs);
            let peak = pcm.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            assert!(peak < 0.98, "{name} clips: peak {peak}");
            let rms = (pcm.iter().map(|s| s * s).sum::<f32>() / pcm.len() as f32).sqrt();
            assert!(rms > 0.1, "{name} too quiet: rms {rms}");
        }
    }
}
