// This is a generic trait, which all audio channels should implement. It is
// primarily meant for use with debug features that display data about the
// audio channels in realtime.

use super::RingBuffer;

pub trait AudioChannelState {
    fn name(&self) -> String;
    fn sample_buffer(&self) -> &RingBuffer;
    fn min_sample(&self) -> i16 {return i16::MIN;}
    fn max_sample(&self) -> i16 {return i16::MAX;}
    fn record_current_output(&mut self);
    fn muted(&self) -> bool;
    fn mute(&mut self);
    fn unmute(&mut self);
}