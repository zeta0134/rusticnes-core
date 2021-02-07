// This is a generic trait, which all audio channels should implement. It is
// primarily meant for use with debug features that display data about the
// audio channels in realtime.

use super::RingBuffer;

#[derive(Clone)]
pub enum PlaybackRate {
    FundamentalFrequency { frequency: f64 },
    LfsrRate { index: usize, max: usize },
    SampleRate { frequency: f64 },
}

#[derive(Clone)]
pub enum Volume {
    VolumeIndex { index: usize, max: usize },
}

#[derive(Clone)]
pub enum Timbre {
    DutyIndex { index: usize, max: usize },
    LsfrMode { index: usize, max: usize },
    PatchIndex { index: usize, max: usize },
}

pub trait AudioChannelState {
    fn name(&self) -> String;
    fn sample_buffer(&self) -> &RingBuffer;
    fn min_sample(&self) -> i16 {return i16::MIN;}
    fn max_sample(&self) -> i16 {return i16::MAX;}
    fn record_current_output(&mut self);
    fn muted(&self) -> bool;
    fn mute(&mut self);
    fn unmute(&mut self);

    fn playing(&self) -> bool { return false; }
    fn rate(&self) -> PlaybackRate { return PlaybackRate::SampleRate{frequency: 0.0}; }
    fn volume(&self) -> Option<Volume> {return None}
    fn timbre(&self) -> Option<Timbre> {return None}
}