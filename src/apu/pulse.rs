use super::length_counter::LengthCounterState;
use super::volume_envelope::VolumeEnvelopeState;

pub struct PulseChannelState {
    pub debug_disable: bool,
    pub debug_buffer: Vec<i16>,
    pub envelope: VolumeEnvelopeState,
    pub length_counter: LengthCounterState,

    // Frequency Sweep
    pub sweep_enabled: bool,
    pub sweep_period: u8,
    pub sweep_divider: u8,
    pub sweep_negate: bool,
    pub sweep_shift: u8,
    pub sweep_reload: bool,
    // Variance between Pulse 1 and Pulse 2 causes negation to work slightly differently
    pub sweep_ones_compliment: bool,

    pub duty: u8,
    pub sequence_counter: u8,
    pub period_initial: u16,
    pub period_current: u16,

}

impl PulseChannelState {
    pub fn new(sweep_ones_compliment: bool) -> PulseChannelState {
        return PulseChannelState {
            debug_disable: false,
            debug_buffer: vec!(0i16; 4096),
            envelope: VolumeEnvelopeState::new(),
            length_counter: LengthCounterState::new(),

            // Frequency Sweep
            sweep_enabled: false,
            sweep_period: 0,
            sweep_divider: 0,
            sweep_negate: false,
            sweep_shift: 0,
            sweep_reload: false,
            // Variance between Pulse 1 and Pulse 2 causes negation to work slightly differently
            sweep_ones_compliment: sweep_ones_compliment,

            duty: 0b0000_0001,
            sequence_counter: 0,
            period_initial: 0,
            period_current: 0,
        }
    }

    pub fn clock(&mut self) {
        if self.period_current == 0 {
            // Reset the period timer, and clock the waveform generator
            self.period_current = self.period_initial;

            // The sequence counter starts at zero, but counts downwards, resulting in an odd
            // lookup sequence of 0, 7, 6, 5, 4, 3, 2, 1
            if self.sequence_counter == 0 {
                self.sequence_counter = 7;
            } else {
                self.sequence_counter -= 1;
            }
        } else {
            self.period_current -= 1;
        }
    }

    pub fn output(&self) -> i16 {
        if self.length_counter.length > 0 {
            let target_period = self.target_period();
            if target_period > 0x7FF || self.period_initial < 8 {
                // Sweep unit mutes the channel, because the period is out of range
                return 0;
            } else {
                let mut sample = ((self.duty >> self.sequence_counter) & 0b1) as i16;
                sample *= self.envelope.current_volume() as i16;
                return sample;
            }
        } else {
            return 0;
        }
    }

    pub fn target_period(&self) -> u16 {
        let change_amount = self.period_initial >> self.sweep_shift;
        if self.sweep_negate {
            if self.sweep_ones_compliment {
                if self.sweep_shift == 0 || self.period_initial == 0 {
                    // Special case: in one's compliment mode, this would overflow to
                    // 0xFFFF, but that's not what real hardware appears to do. This solves
                    // a muting bug with negate-mode sweep on Pulse 1 in some publishers
                    // games.
                    return 0;
                }
                return self.period_initial - change_amount - 1;
            } else {
                return self.period_initial - change_amount;
            }
        } else {
            return self.period_initial + change_amount;
        }
    }

    pub fn update_sweep(&mut self) {
        let target_period = self.target_period();
        if self.sweep_divider == 0 && self.sweep_enabled && self.sweep_shift != 0
        && target_period <= 0x7FF && self.period_initial >= 8 {
            self.period_initial = target_period;
        }
        if self.sweep_divider == 0 || self.sweep_reload {
            self.sweep_divider = self.sweep_period;
            self.sweep_reload = false;
        } else {
            self.sweep_divider -= 1;
        }
    }
}