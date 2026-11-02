//! Songs on the toolkit's built-in synth (synthie): a [`Tune`] is a tracker [`Song`] plus the
//! patch each synth channel starts on. Music streams live through kira ([`Tune::sound_data`]);
//! jingles and snapshots render the very same setup offline ([`Tune::render`]).

use game_toolkit::prelude::synth::{AudioEvent, ChannelNo, Song, SongPlayer, SynthParams};
use game_toolkit::prelude::synthie::kira::SynthSoundData;
use game_toolkit::prelude::synthie::prelude::{
    NUM_CHANNELS, SequencedProcessor, SynthProcessor, VoiceNormalization,
};

/// Output gain after the mix. Voices aren't divided by the polyphony (every lane is
/// monophonic, so a lone note should play at its patch's level); this keeps the sum of a
/// lead, chords, bass and drums clear of the synth's final clamp (the in-game music peaks
/// at about 0.9). `songs::tests::music_never_clips` checks it.
pub const MASTER_GAIN: f32 = 0.38;

/// A song and the patch each of its channels plays.
#[derive(Clone, Debug)]
pub struct Tune {
    pub song: Song,
    /// Channel `i` starts on `patches[i]`; at most [`NUM_CHANNELS`].
    pub patches: Vec<SynthParams>,
}

impl Tune {
    /// `song`'s lane on channel `i` plays `patches[i]`.
    ///
    /// # Panics
    ///
    /// With more patches than synth channels. Drum-only lanes may share any channel: drum
    /// hits don't use its voices.
    pub fn new(song: Song, patches: Vec<SynthParams>) -> Self {
        assert!(
            patches.len() <= NUM_CHANNELS,
            "at most {NUM_CHANNELS} channels"
        );
        Self { song, patches }
    }

    /// The tune as a kira sound for [`Audio::play_synth_sound`]. Loops if the song does.
    ///
    /// [`Audio::play_synth_sound`]: game_toolkit::prelude::Audio::play_synth_sound
    pub fn sound_data(&self) -> SynthSoundData<SongPlayer> {
        SynthSoundData::new()
            .with_patches(self.patches.iter().cloned())
            .with_voice_normalization(VoiceNormalization::Off)
            .with_master_gain(MASTER_GAIN)
            .with_sequencer(SongPlayer::new(self.song.clone()))
    }

    /// Render `secs` of the tune at `sample_rate`, mixed exactly as [`Tune::sound_data`]
    /// plays it.
    pub fn render(&self, sample_rate: u32, secs: f32) -> Vec<f32> {
        let mut processor = SynthProcessor::<NUM_CHANNELS>::new(sample_rate as f32);
        processor.set_voice_normalization(VoiceNormalization::Off);
        processor.set_master_gain(MASTER_GAIN);
        let setup: Vec<AudioEvent> = self
            .patches
            .iter()
            .enumerate()
            .map(|(i, p)| AudioEvent::LoadPatchChannel(ChannelNo(i as u8), Box::new(p.clone())))
            .collect();
        let mut seq = SequencedProcessor::new(processor, SongPlayer::new(self.song.clone()));
        let mut out = vec![0.0; (secs * sample_rate as f32) as usize];
        seq.process_block(&setup, &mut out, 1);
        out
    }

    /// One pass of the song plus `tail` seconds for the notes to ring out: a jingle.
    pub fn render_once(&self, sample_rate: u32, tail: f32) -> Vec<f32> {
        let once = Self {
            song: self.song.clone().one_shot(),
            patches: self.patches.clone(),
        };
        once.render(sample_rate, self.song.duration_secs() + tail)
    }
}
