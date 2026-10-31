//! The soundtrack, in tracker notation (see [`super::tracker`]).

use super::synth::Wave;
use super::tracker::{Channel, Instrument, Song};

fn lead() -> Instrument {
    Instrument {
        sustain: 0.55,
        decay: 5.0,
        vibrato: (0.25, 6.0, 0.18),
        ..Instrument::new(Wave::Pulse(0.25), 0.32)
    }
}

fn bass() -> Instrument {
    Instrument {
        sustain: 0.8,
        decay: 3.0,
        release: 30.0,
        ..Instrument::new(Wave::Triangle, 0.55)
    }
}

fn chords() -> Instrument {
    Instrument {
        sustain: 0.3,
        decay: 12.0,
        release: 25.0,
        ..Instrument::new(Wave::Pulse(0.125), 0.16)
    }
}

/// In-game: bouncy C major, eighth-note rows at 150 bpm.
pub fn game() -> Song {
    Song {
        rpm: 300.0,
        looped: true,
        gain: 1.0,
        channels: vec![
            Channel::new(
                Some(lead()),
                "E5 . G5 E5 C5 . D5 E5 | C5 . A4 . C5 E5 D5 C5 |
                 A4 . C5 A4 F4 . G4 A4 | B4 . D5 . G5 . - . |
                 E5 G5 C6 . B5 G5 E5 . | A5 . G5 E5 C5 . E5 . |
                 F5 E5 D5 C5 A4 . C5 . | D5 . B4 G4 D5 E5 F5 D5 |",
            ),
            Channel::new(
                Some(chords()),
                "- C4^047 - C4^047 - C4^047 - C4^047 | - A3^037 - A3^037 - A3^037 - A3^037 |
                 - F3^047 - F3^047 - F3^047 - F3^047 | - G3^047 - G3^047 - G3^047 - G3^047 |
                 - C4^047 - C4^047 - C4^047 - C4^047 | - A3^037 - A3^037 - A3^037 - A3^037 |
                 - F3^047 - F3^047 - F3^047 - F3^047 | - G3^047 - G3^047 - G3^04a - G3^04a |",
            ),
            Channel::new(
                Some(bass()),
                "C2 C3 C2 C3 C2 C3 G1 B1 | A1 A2 A1 A2 A1 A2 E2 G2 |
                 F1 F2 F1 F2 F1 F2 C2 E2 | G1 G2 G1 G2 G1 G2 D2 B1 |
                 C2 C3 C2 C3 C2 C3 G1 B1 | A1 A2 A1 A2 A1 A2 E2 G2 |
                 F1 F2 F1 F2 F1 F2 C2 E2 | G1 G2 G1 G2 D2 D2 B1 G1 |",
            ),
            Channel::new(
                None,
                "k h s h k k s h | k h s h k k s h | k h s h k k s h | k h s h k s s s |
                 k h s h k k s h | k h s h k k s h | k h s h k k s h | k h s h s s s s |",
            ),
        ],
    }
}

/// Title screen: a smug little ii-V-I lounge loop.
pub fn title() -> Song {
    let lead = Instrument {
        vibrato: (0.3, 5.0, 0.12),
        ..lead()
    };
    Song {
        rpm: 232.0,
        looped: true,
        gain: 1.0,
        channels: vec![
            Channel::new(
                Some(lead),
                "F5 . E5 D5 . A4 C5 . | B4 . D5 F5 . E5 D5 . |
                 E5 . . G5 . . E5 C5 | D5 . . . - . . . |
                 A5 . G5 F5 . D5 F5 . | G5 . F5 D5 . B4 D5 . |
                 C5 . E5 G5 . C#5 E5 G5 | F5 . E5 . D5 . B4 - |",
            ),
            Channel::new(
                Some(chords()),
                "D4^37a . . . D4^37a . . - | G3^47a . . . G3^47a . . - |
                 C4^47b . . . C4^47b . . - | C4^47b . . . C4^47b . . - |
                 D4^37a . . . D4^37a . . - | G3^47a . . . G3^47a . . - |
                 C4^47b . . . A3^47a . . - | D4^37a . . . G3^47a . . - |",
            ),
            Channel::new(
                Some(bass()),
                "D2 . F2 . A2 . C3 . | G1 . B1 . D2 . F2 . |
                 C2 . E2 . G2 . B2 . | C2 . G1 . A1 . B1 . |
                 D2 . F2 . A2 . C3 . | G1 . B1 . D2 . F2 . |
                 C2 . E2 . A1 . C#2 . | D2 . A1 . G1 . B1 . |",
            ),
            Channel::new(
                None,
                "k . h . s . h h | k . h . s . h h | k . h . s . h h | k . h . s . h s |
                 k . h . s . h h | k . h . s . h h | k . h . s . h h | k . s . s s s s |",
            ),
        ],
    }
}

/// "Get ready!"
pub fn level_start() -> Song {
    Song {
        rpm: 600.0,
        looped: false,
        gain: 1.0,
        channels: vec![
            Channel::new(Some(lead()), "G4 C5 E5 G5 . . E5 G5 . . . -"),
            Channel::new(Some(bass()), "C2 . . . . . C3 . . . . -"),
            Channel::new(None, "k . . . s . k . s . . ."),
        ],
    }
}

/// Every cookie accepted.
pub fn level_clear() -> Song {
    Song {
        rpm: 480.0,
        looped: false,
        gain: 1.0,
        channels: vec![
            Channel::new(
                Some(lead()),
                "C5 E5 G5 C6 . . G5 C6 . . E6 . D6 . C6 . . . . -",
            ),
            Channel::new(
                Some(chords()),
                "C4^047 . . . . . . . . . F4^047 . G4^047 . C4^047 . . . . -",
            ),
            Channel::new(
                Some(bass()),
                "C2 . . . G2 . . . C3 . F2 . G2 . C2 . . . . -",
            ),
            Channel::new(None, "k . . . s . k . s . k . s . k . . . . ."),
        ],
    }
}

/// Session expired.
pub fn game_over() -> Song {
    let lead = Instrument {
        vibrato: (0.4, 5.0, 0.2),
        sustain: 0.7,
        ..Instrument::new(Wave::Pulse(0.5), 0.3)
    };
    Song {
        rpm: 200.0,
        looped: false,
        gain: 1.0,
        channels: vec![
            Channel::new(Some(lead), "C5 . G4 . Eb4 . D4 . C4 . . . . -"),
            Channel::new(Some(bass()), "C3 . . . G2 . . . C2 . . . . -"),
            Channel::new(None, "k . . . k . . . s . . . . ."),
        ],
    }
}

/// The sad trombone after a "D'oh!".
pub fn sad_trombone() -> Song {
    let horn = Instrument {
        wave: Wave::Saw,
        volume: 0.35,
        attack: 0.03,
        decay: 2.0,
        sustain: 0.8,
        release: 8.0,
        vibrato: (0.45, 6.5, 0.15),
    };
    Song {
        rpm: 320.0,
        looped: false,
        gain: 1.0,
        channels: vec![Channel::new(Some(horn), "G3 . F#3 . F3 . E3 . . . -")],
    }
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
pub fn cracktro() -> Song {
    let lead = "A4 . C5 . E5 . D5 C5 | C5 . A4 . F4 . G4 A4 |
                G4 . E4 . G4 . C5 . | B4 . A4 . G4 . - . |
                E5 . A5 . G5 . E5 D5 | C5 . F5 . E5 . C5 A4 |
                G4 . C5 . E5 . G5 . | F5 . E5 . D5 . B4 - |";
    let voice = Instrument {
        attack: 0.01,
        decay: 3.0,
        sustain: 0.5,
        release: 6.0,
        vibrato: (0.2, 5.0, 0.2),
        ..Instrument::new(Wave::Pulse(0.5), 0.2)
    };
    let echo = Instrument {
        volume: 0.07,
        ..voice
    };
    let pad = Instrument {
        sustain: 0.5,
        decay: 2.0,
        release: 4.0,
        ..Instrument::new(Wave::Pulse(0.125), 0.09)
    };
    let soft_bass = Instrument {
        volume: 0.45,
        release: 10.0,
        ..bass()
    };
    Song {
        rpm: 200.0,
        looped: true,
        gain: 0.9,
        channels: vec![
            Channel::new(Some(voice), lead),
            Channel::new(Some(echo), &delayed(lead, 3)),
            Channel::new(
                Some(pad),
                "A3^037 . . . . . . . | F3^047 . . . . . . . | C4^047 . . . . . . . | G3^047 . . . . . . . |
                 A3^037 . . . . . . . | F3^047 . . . . . . . | C4^047 . . . . . . . | G3^047 . . . . . . . |",
            ),
            Channel::new(
                Some(soft_bass),
                "A1 . A2 . A1 . E2 . | F1 . F2 . F1 . C2 . | C2 . C3 . C2 . G1 . | G1 . G2 . G1 . D2 . |
                 A1 . A2 . A1 . E2 . | F1 . F2 . F1 . C2 . | C2 . C3 . C2 . G1 . | G1 . G2 . G1 . D2 . |",
            ),
            Channel::new(
                None,
                "k . . . s . . . | k . . . s . . . | k . . . s . . . | k . . k s . . . |
                 k . . . s . . . | k . . . s . . . | k . . . s . . . | k . . k s . s . |",
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn echo_is_the_lead_rotated() {
        assert_eq!(super::delayed("A4 . | B4 C5", 1), "C5 A4 . B4");
    }

    #[test]
    fn all_songs_have_equal_channel_lengths() {
        for song in [super::game(), super::title(), super::cracktro()] {
            let lens: Vec<usize> = song.channels.iter().map(|c| c.rows.len()).collect();
            assert!(lens.windows(2).all(|w| w[0] == w[1]), "{lens:?}");
        }
    }
}
