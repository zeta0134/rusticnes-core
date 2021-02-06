use super::length_counter::LengthCounterState;
use super::volume_envelope::VolumeEnvelopeState;

pub struct NoiseChannelState {
    pub debug_disable: bool,
    pub debug_buffer: Vec<i16>,
    pub length: u8,
    pub length_halt_flag: bool,

    pub envelope: VolumeEnvelopeState,
    pub length_counter: LengthCounterState,

    pub mode: u8,
    pub period_initial: u16,
    pub period_current: u16,

    // Actually a 15-bit register
    pub shift_register: u16,
}

impl NoiseChannelState {
    pub fn new() -> NoiseChannelState {
        return NoiseChannelState {
            debug_disable: false,
            debug_buffer: vec!(0i16; 4096),
            length: 0,
            length_halt_flag: false,

            envelope: VolumeEnvelopeState::new(),
            length_counter: LengthCounterState::new(),
            mode: 0,
            period_initial: 0,
            period_current: 0,

            // Actually a 15-bit register
            shift_register: 1,
        }
    }

    pub fn clock(&mut self) {
        if self.period_current == 0 {
            self.period_current = self.period_initial;

            let mut feedback = self.shift_register & 0b1;
            if self.mode == 1 {
                feedback ^= (self.shift_register >> 6) & 0b1;
            } else {
                feedback ^= (self.shift_register >> 1) & 0b1;
            }
            self.shift_register = self.shift_register >> 1;
            self.shift_register |= feedback << 14;
        } else {
            self.period_current -= 1;
        }
    }

    pub fn output(&self) -> i16 {
        if self.length_counter.length > 0 {
            let mut sample = (self.shift_register & 0b1) as i16;
            sample *= self.envelope.current_volume() as i16;
            return sample;
        } else {
            return 0;
        }
    }
}