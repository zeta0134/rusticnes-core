use mmc::mapper::Mapper;

use std::fs::OpenOptions;
use std::io::prelude::*;

pub struct VolumeEnvelopeState {
    // Volume Envelope
    pub volume_register: u8,
    pub decay: u8,
    pub divider: u8,
    pub enabled: bool,
    pub looping: bool,
    pub start_flag: bool,
}

impl VolumeEnvelopeState {
    pub fn new() -> VolumeEnvelopeState {
        return VolumeEnvelopeState {
            volume_register: 0,
            decay: 0,
            divider: 0,
            enabled: false,
            looping: false,
            start_flag: false,
        }
    }

    pub fn current_volume(&self) -> u8 {
        if self.enabled {
            return self.decay;
        } else {
            return self.volume_register;
        }
    }

    pub fn clock(&mut self) {
        if self.start_flag {
            self.decay = 15;
            self.start_flag = false;
            self.divider = self.volume_register;
        } else {
            // Clock the divider
            if self.divider == 0 {
                self.divider = self.volume_register;
                if self.decay > 0 {
                    self.decay -= 1;
                } else {
                    if self.looping {
                        self.decay = 15;
                    }
                }
            } else {
                self.divider = self.divider - 1;
            }
        }
    }
}

pub struct LengthCounterState {
    pub length: u8,
    pub halt_flag: bool,
    pub channel_enabled: bool,
}

impl LengthCounterState{
    pub fn new() -> LengthCounterState {
        return LengthCounterState {
            length: 0,
            halt_flag: false,
            channel_enabled: false,
        }
    }

    pub fn clock(&mut self) {
        if self.channel_enabled {
            if self.length > 0 && !(self.halt_flag) {
                self.length -= 1;
            }
        } else {
            self.length = 0;
        }
    }

    pub fn set_length(&mut self, index: u8) {
        if self.channel_enabled {
            let table = [
                10, 254, 20,  2, 40,  4, 80,  6, 160,  8, 60, 10, 14, 12, 26, 14,
                12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30];
            self.length = table[index as usize];
        } else {
            self.length = 0
        }
    }
}

pub struct PulseChannelState {
    pub debug_disable: bool,
    pub debug_buffer: [u16; 4096],
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
            debug_buffer: [0u16; 4096],
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

    pub fn output(&self) -> u16 {
        if self.length_counter.length > 0 {
            let target_period = self.target_period();
            if target_period > 0x7FF || self.period_initial < 8 {
                // Sweep unit mutes the channel, because the period is out of range
                return 0;
            } else {
                let mut sample = ((self.duty >> self.sequence_counter) & 0b1) as u16;
                sample *= self.envelope.current_volume() as u16;
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

pub struct TriangleChannelState {
    pub debug_disable: bool,
    pub debug_buffer: [u16; 4096],
    pub length_counter: LengthCounterState,

    pub control_flag: bool,
    pub linear_reload_flag: bool,
    pub linear_counter_initial: u8,
    pub linear_counter_current: u8,

    pub sequence_counter: u8,
    pub period_initial: u16,
    pub period_current: u16,
    pub length: u8,
}

impl TriangleChannelState {
    pub fn new() -> TriangleChannelState {
        return TriangleChannelState {
            debug_disable: false,
            debug_buffer: [0u16; 4096],
            length_counter: LengthCounterState::new(),
            control_flag: false,
            linear_reload_flag: false,
            linear_counter_initial: 0,
            linear_counter_current: 0,

            sequence_counter: 0,
            period_initial: 0,
            period_current: 0,
            length: 0,
        }
    }

    pub fn update_linear_counter(&mut self) {
        if self.linear_reload_flag {
            self.linear_counter_current = self.linear_counter_initial;
        } else {
            if self.linear_counter_current > 0 {
                self.linear_counter_current -= 1;
            }
        }
        if !(self.control_flag) {
            self.linear_reload_flag = false;
        }
    }

    pub fn clock(&mut self) {
        if self.linear_counter_current != 0 {
            if self.period_current == 0 {
                // Reset the period timer, and clock the waveform generator
                self.period_current = self.period_initial;

                // The sequence counter starts at zero, but counts downwards, resulting in an odd
                // lookup sequence of 0, 7, 6, 5, 4, 3, 2, 1
                if self.sequence_counter >= 31 {
                    self.sequence_counter = 0;
                } else {
                    self.sequence_counter += 1;
                }
            } else {
                self.period_current -= 1;
            }
        }
    }

    pub fn output(&self) -> u16 {
        if self.length_counter.length > 0 {
            if self.period_initial <= 2 {
                // This frequency is so high that the hardware mixer can't keep up, and effectively
                // receives 7.5. We'll just return 7 here (close enough). Some games use this
                // to silence the channel, and returning 7 emulates the resulting clicks and pops.
                return 7;
            } else {
                let triangle_sequence = [15,14,13,12,11,10,9,8,7,6,5,4,3,2,1,0,
                                         0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15];
                return triangle_sequence[self.sequence_counter as usize];
            }
        } else {
            return 0;
        }
    }
}

pub struct NoiseChannelState {
    pub debug_disable: bool,
    pub debug_buffer: [u16; 4096],
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
            debug_buffer: [0u16; 4096],
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

    pub fn output(&self) -> u16 {
        if self.length_counter.length > 0 {
            let mut sample = (self.shift_register & 0b1) as u16;
            sample *= self.envelope.current_volume() as u16;
            return sample;
        } else {
            return 0;
        }
    }
}

pub struct DmcState {
    pub debug_disable: bool,
    pub debug_buffer: [u16; 4096],

    pub looping: bool,
    pub period_initial: u16,
    pub period_current: u16,
    pub output_level: u8,
    pub starting_address: u16,
    pub sample_length: u16,

    pub current_address: u16,
    pub sample_buffer: u8,
    pub shift_register: u8,
    pub sample_buffer_empty: bool,
    pub bits_remaining: u8,
    pub bytes_remaining: u16,
    pub silence_flag: bool,

    pub interrupt_enabled: bool,
    pub interrupt_flag: bool,
}

impl DmcState {
    pub fn new() -> DmcState {
        return DmcState {
            debug_disable: false,
            debug_buffer: [0u16; 4096],

            looping: false,
            period_initial: 0,
            period_current: 0,
            output_level: 0,
            starting_address: 0,
            sample_length: 0,

            current_address: 0,
            sample_buffer: 0,
            shift_register: 0,
            sample_buffer_empty: true,
            bits_remaining: 8,
            bytes_remaining: 0,
            silence_flag: false,
            interrupt_enabled: true,
            interrupt_flag: false,
        }
    }

    pub fn debug_status(&self) -> String {
        return format!("Rate: {:3} - Divisor: {:3} - Start: {:04X} - Current: {:04X} - Length: {:4} - R.Bytes: {:4} - R.Bits: {:1}", 
            self.period_initial, self.period_current, self.starting_address, self.current_address, self.sample_length,
            self.bytes_remaining, self.bits_remaining);
    }

    pub fn read_next_sample(&mut self, mapper: &mut Mapper) {
        self.sample_buffer = mapper.read_byte(0x8000 | (self.current_address & 0x7FFF));
        self.current_address = self.current_address.wrapping_add(1);
        self.bytes_remaining -= 1;
        if self.bytes_remaining == 0 && self.looping {
            self.current_address = self.starting_address;
            self.bytes_remaining = self.sample_length;
        } else {
            self.interrupt_flag = true;
        }
        self.sample_buffer_empty = false;
    }

    pub fn begin_output_cycle(&mut self) {
        self.bits_remaining = 8;
        if self.sample_buffer_empty {
            self.silence_flag = true;
        } else {
            self.silence_flag = false;
            self.shift_register = self.sample_buffer;
            self.sample_buffer_empty = true;
        }
    }

    pub fn update_output_unit(&mut self) {
        if !(self.silence_flag) {
            let mut target_output = self.output_level;
            if (self.shift_register & 0b1) == 0 {
                if self.output_level >= 2 {
                    target_output -= 2;
                }
            } else  {
                if self.output_level <= 125 {
                    target_output += 2;
                }
            }
            self.output_level = target_output;
        }
        self.shift_register = self.shift_register >> 1;
        self.bits_remaining -= 1;
        if self.bits_remaining == 0 {
            self.begin_output_cycle();
        }
    }

    pub fn clock(&mut self, mapper: &mut Mapper) {
        if self.period_current == 0 {
            self.period_current = self.period_initial;
            self.update_output_unit();
        } else {
            self.period_current -= 1;
        }
        if self.sample_buffer_empty && self.bytes_remaining > 0 {
            self.read_next_sample(mapper);
        }
    }

    pub fn output(&self) -> u16 {
        return self.output_level as u16;
    }
}

pub struct ApuState {
    pub current_cycle: u64,

    pub frame_sequencer_mode: u8,
    pub frame_sequencer: u16,
    pub frame_reset_delay: u8,

    pub frame_interrupt: bool,
    pub disable_interrupt: bool,

    pub pulse_1: PulseChannelState,
    pub pulse_2: PulseChannelState,
    pub triangle: TriangleChannelState,
    pub noise: NoiseChannelState,
    pub dmc: DmcState,

    pub sample_buffer: [u16; 4096],
    pub output_buffer: [u16; 4096],
    pub buffer_full: bool,
    pub sample_rate: u64,
    pub cpu_clock_rate: u64,
    pub buffer_index: usize,
    pub generated_samples: u64,
    pub next_sample_at: u64,
}

impl ApuState {
    pub fn new() -> ApuState {

        return ApuState {
            current_cycle: 0,
            frame_sequencer_mode: 0,
            frame_sequencer: 0,
            frame_reset_delay: 0,
            frame_interrupt: false,
            disable_interrupt: false,
            pulse_1: PulseChannelState::new(true),
            pulse_2: PulseChannelState::new(false),
            triangle: TriangleChannelState::new(),
            noise: NoiseChannelState::new(),
            dmc: DmcState::new(),
            sample_buffer: [0u16; 4096],
            output_buffer: [0u16; 4096],
            buffer_full: false,
            sample_rate: 44100,
            cpu_clock_rate: 1_786_860,
            buffer_index: 0,
            generated_samples: 0,
            next_sample_at: 0,
        }
    }

    pub fn read_register(&mut self, address: u16) -> u8 {
        match address {
            0x4015 => {
                let mut status = 0;
                if self.pulse_1.length_counter.length > 0 {
                    status += 0b0000_0001;
                }
                if self.pulse_2.length_counter.length > 0 {
                    status += 0b0000_0010;
                }
                if self.triangle.length_counter.length > 0 {
                    status += 0b0000_0100;
                }
                if self.noise.length_counter.length > 0 {
                    status += 0b0000_1000;
                }
                if self.dmc.bytes_remaining > 0 {
                    status += 0b0001_0000;
                }

                if self.frame_interrupt {
                    status += 0b0100_0000;
                }
                if self.dmc.interrupt_flag {
                    status += 0b1000_0000;   
                }
                // Reading from this register resets frame_interrupt:
                self.frame_interrupt = false;
                return status;
            },
            _ => return 0
        }
    }

    pub fn write_register(&mut self, address: u16, data: u8) {
        let duty_table = [
            0b1000_0000,
            0b1100_0000,
            0b1111_0000,
            0b0011_1111,
        ];
        match address {
            // Pulse Channel 1
            0x4000 => {
                let duty_index =      (data & 0b1100_0000) >> 6;
                let length_disable =  (data & 0b0010_0000) != 0;
                let constant_volume = (data & 0b0001_0000) != 0;

                self.pulse_1.duty = duty_table[duty_index as usize];
                self.pulse_1.length_counter.halt_flag = length_disable;
                self.pulse_1.envelope.looping = length_disable;
                self.pulse_1.envelope.enabled = !(constant_volume);
                self.pulse_1.envelope.volume_register = data & 0b0000_1111;
            },
            0x4001 => {
                self.pulse_1.sweep_enabled =  (data & 0b1000_0000) != 0;
                self.pulse_1.sweep_period =   (data & 0b0111_0000) >> 4;
                self.pulse_1.sweep_negate =   (data & 0b0000_1000) != 0;
                self.pulse_1.sweep_shift =     data & 0b0000_0111;
                self.pulse_1.sweep_reload = true;
            },
            0x4002 => {
                let period_low = data as u16;
                self.pulse_1.period_initial = (self.pulse_1.period_initial & 0xFF00) | period_low
            },
            0x4003 => {
                let period_high =  ((data & 0b0000_0111) as u16) << 8;
                let length_index = (data & 0b1111_1000) >> 3;

                self.pulse_1.period_initial = (self.pulse_1.period_initial & 0x00FF) | period_high;
                self.pulse_1.length_counter.set_length(length_index);

                // Start this note
                self.pulse_1.sequence_counter = 0;
                self.pulse_1.envelope.start_flag = true;
            },

            // Pulse Channel 2
            0x4004 => {
                let duty_index =      (data & 0b1100_0000) >> 6;
                let length_disable =  (data & 0b0010_0000) != 0;
                let constant_volume = (data & 0b0001_0000) != 0;

                self.pulse_2.duty = duty_table[duty_index as usize];
                self.pulse_2.length_counter.halt_flag = length_disable;
                self.pulse_2.envelope.looping = length_disable;
                self.pulse_2.envelope.enabled = !(constant_volume);
                self.pulse_2.envelope.volume_register = data & 0b0000_1111;
            },
            0x4005 => {
                self.pulse_2.sweep_enabled =  (data & 0b1000_0000) != 0;
                self.pulse_2.sweep_period =   (data & 0b0111_0000) >> 4;
                self.pulse_2.sweep_negate =   (data & 0b0000_1000) != 0;
                self.pulse_2.sweep_shift =     data & 0b0000_0111;
                self.pulse_2.sweep_reload = true;
            },
            0x4006 => {
                let period_low = data as u16;
                self.pulse_2.period_initial = (self.pulse_2.period_initial & 0xFF00) | period_low
            },
            0x4007 => {
                let period_high =  ((data & 0b0000_0111) as u16) << 8;
                let length_index =  (data & 0b1111_1000) >> 3;

                self.pulse_2.period_initial = (self.pulse_2.period_initial & 0x00FF) | period_high;
                self.pulse_2.length_counter.set_length(length_index);

                // Start this note
                self.pulse_2.sequence_counter = 0;
                self.pulse_2.envelope.start_flag = true;
            },

            // Triangle Channel
            0x4008 => {
                self.triangle.control_flag           = (data & 0b1000_0000) != 0;
                self.triangle.length_counter.halt_flag = self.triangle.control_flag;
                self.triangle.linear_counter_initial =  data & 0b0111_1111;
            },
            0x400A => {
                let period_low = data as u16;
                self.triangle.period_initial = (self.triangle.period_initial & 0xFF00) | period_low
            },
            0x400B => {
                let period_high =  ((data & 0b0000_0111) as u16) << 8;
                let length_index =  (data & 0b1111_1000) >> 3;

                self.triangle.period_initial = (self.triangle.period_initial & 0x00FF) | period_high;
                self.triangle.length_counter.set_length(length_index);

                // Start this note
                self.triangle.linear_reload_flag = true;
            },

            // Noise Channel
            0x400C => {
                let length_disable =  (data & 0b0010_0000) != 0;
                let constant_volume = (data & 0b0001_0000) != 0;

                self.noise.length_counter.halt_flag = length_disable;
                self.noise.envelope.looping = length_disable;
                self.noise.envelope.enabled = !(constant_volume);
                self.noise.envelope.volume_register = data & 0b0000_1111;
            },
            0x400E => {
                let noise_period = [
                    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068];

                let mode =        (data & 0b1000_0000) >> 7;
                let period_index = data & 0b0000_1111;
                self.noise.mode = mode;
                self.noise.period_initial = noise_period[period_index as usize];
            },
            0x400F => {
                let length_index = (data & 0b1111_1000) >> 3;
                self.noise.length_counter.set_length(length_index);

                // Restart the envelope
                self.noise.envelope.start_flag = true;
            },

            // DMC Channel
            0x4010 => {
                let period_table = [
                    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106,  84,  72,  54];
                self.dmc.looping = (data & 0b0100_0000) != 0;
                self.dmc.interrupt_enabled = (data & 0b1000_0000) != 0;
                if !self.dmc.interrupt_enabled {
                    // If the enable bit was cleared, clear the flag also.
                    self.dmc.interrupt_flag = false;
                }
                let period_index = data & 0b0000_1111;
                self.dmc.period_initial = period_table[period_index as usize] / 2;
            },
            0x4011 => {
                self.dmc.output_level = data & 0b0111_1111;
            },
            0x4012 => {
                self.dmc.starting_address = 0xC000 + (data as u16 * 64);
            },
            0x4013 => {
                self.dmc.sample_length = (data as u16 * 16) + 1;
            },

            // Status / Enabled
            0x4015 => {
                self.pulse_1.length_counter.channel_enabled  = (data & 0b0001) != 0;
                self.pulse_2.length_counter.channel_enabled  = (data & 0b0010) != 0;
                self.triangle.length_counter.channel_enabled = (data & 0b0100) != 0;
                self.noise.length_counter.channel_enabled    = (data & 0b1000) != 0;

                if !(self.pulse_1.length_counter.channel_enabled) {
                    self.pulse_1.length_counter.length = 0;
                }
                if !(self.pulse_2.length_counter.channel_enabled) {
                    self.pulse_2.length_counter.length = 0;
                }
                if !(self.triangle.length_counter.channel_enabled) {
                    self.triangle.length_counter.length = 0;
                }
                if !(self.noise.length_counter.channel_enabled) {
                    self.noise.length_counter.length = 0;
                }

                let dmc_enable = (data & 0b1_0000) != 0;
                if !(dmc_enable) {
                    self.dmc.bytes_remaining = 0;
                }
                if dmc_enable && self.dmc.bytes_remaining == 0 {
                    self.dmc.current_address = self.dmc.starting_address;
                    self.dmc.bytes_remaining = self.dmc.sample_length;
                }
                self.dmc.interrupt_flag = false;
            }

            // Frame Counter / Interrupts
            0x4017 => {
                self.frame_sequencer_mode = (data & 0b1000_0000) >> 7;
                self.disable_interrupt =    (data & 0b0100_0000) != 0;
                self.frame_reset_delay = 4;
                // If interrupts are disabled, clear the flag too:
                if self.disable_interrupt {
                    self.frame_interrupt = false;
                }
            }

            _ => ()
        }
    }

    // Note: this uses CPU clocks, NOT APU clocks! It's simpler to represent the half-clock
    // updates this way. Documentation: https://wiki.nesdev.com/w/index.php/APU_Frame_Counter

    pub fn clock_frame_sequencer(&mut self) {
        self.frame_sequencer += 1;

        if self.frame_reset_delay > 0 {
            self.frame_reset_delay -= 1;
            if self.frame_reset_delay == 0 {
                self.frame_sequencer = 0;
                if self.frame_sequencer_mode == 1 {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
            }
        }

        if self.frame_sequencer_mode == 0 {
            // 4-step sequence
            match self.frame_sequencer {
                7457 => self.clock_quarter_frame(),
                14913 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                },
                22371 => self.clock_quarter_frame(),
                29828 => {
                    if !self.disable_interrupt {
                        self.frame_interrupt = true;
                    }
                },
                29829 => {
                    if !self.disable_interrupt {
                        self.frame_interrupt = true;
                    }
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                },
                29830 => {
                    if !self.disable_interrupt {
                        self.frame_interrupt = true;
                    }
                    self.frame_sequencer = 0;
                },
                _ => ()
            }
        } else {
            match self.frame_sequencer {
                // "5-step" sequence (uneven timing)
                7457 => self.clock_quarter_frame(),
                14913 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                },
                22371 => self.clock_quarter_frame(),
                37281 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                },
                37282 => {
                    self.frame_sequencer = 0;
                },
                _ => ()
            }
        }
    }

    pub fn clock_quarter_frame(&mut self) {
        self.pulse_1.envelope.clock();
        self.pulse_2.envelope.clock();
        self.triangle.update_linear_counter();
        self.noise.envelope.clock();
    }

    pub fn clock_half_frame(&mut self) {
        self.pulse_1.update_sweep();
        self.pulse_2.update_sweep();

        self.pulse_1.length_counter.clock();
        self.pulse_2.length_counter.clock();
        self.triangle.length_counter.clock();
        self.noise.length_counter.clock();
    }

    pub fn run_to_cycle(&mut self, target_cycle: u64, mapper: &mut Mapper) {
        while self.current_cycle < target_cycle {
            self.clock_frame_sequencer();

            // Clock the triangle channel once per CPU cycle
            self.triangle.clock();

            // Only clock Pulse channels on every other cycle
            // (Most documentation calls this once per APU cycle)
            if (self.current_cycle & 0b1) == 0 {
                self.pulse_1.clock();
                self.pulse_2.clock();
                self.noise.clock();
                self.dmc.clock(mapper);
            }

            if self.current_cycle >= self.next_sample_at {
                // Mixing? Bah! Just throw the sample in the buffer.
                let mut composite_sample: u16 = 0;
                let pulse_1_sample = self.pulse_1.output();
                self.pulse_1.debug_buffer[self.buffer_index] = pulse_1_sample;
                if !(self.pulse_1.debug_disable) {
                    composite_sample += pulse_1_sample * 512; // Sure, why not?
                }

                let pulse_2_sample = self.pulse_2.output();
                self.pulse_2.debug_buffer[self.buffer_index] = pulse_2_sample;
                if !(self.pulse_2.debug_disable) {
                    composite_sample += pulse_2_sample * 512; // Sure, why not?
                }

                let triangle_sample = self.triangle.output();
                self.triangle.debug_buffer[self.buffer_index] = triangle_sample;
                if !(self.triangle.debug_disable) {
                    composite_sample += triangle_sample * 512; // Sure, why not?
                }

                let noise_sample = self.noise.output();
                self.noise.debug_buffer[self.buffer_index] = noise_sample;
                if !(self.noise.debug_disable) {
                    composite_sample += noise_sample * 512; // Sure, why not?
                }

                let dmc_sample = self.dmc.output();
                self.dmc.debug_buffer[self.buffer_index] = dmc_sample;
                if !(self.dmc.debug_disable) {
                    composite_sample += dmc_sample * 128; // Sure, why not?
                }

                self.sample_buffer[self.buffer_index] = composite_sample;
                self.buffer_index = (self.buffer_index + 1) % self.sample_buffer.len();

                self.generated_samples += 1;
                self.next_sample_at = ((self.generated_samples + 1) * self.cpu_clock_rate) / self.sample_rate;

                if self.buffer_index == 0 {
                    //self.dump_sample_buffer();
                    self.output_buffer.copy_from_slice(&self.sample_buffer);
                    self.buffer_full = true;
                }
            }

            self.current_cycle += 1;
        }
    }

    pub fn dump_sample_buffer(&self) {
        let mut file =
            OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open("audiodump.raw")
            .unwrap();

        // turn our sample buffer into a simple file buffer for output
        let mut buffer = [0u8; 4096 * 2];
        for i in 0 .. 4096 {
            let sample = ((self.sample_buffer[i] as i32) - 32768) as u16;
            buffer[i * 2]     = ((sample & 0xFF00) >> 8) as u8;
            buffer[i * 2 + 1] = ((sample & 0x00FF)     ) as u8;
        }

        let _ = file.write_all(&buffer);
    }

    pub fn irq_signal(&self) -> bool {
        return self.frame_interrupt || self.dmc.interrupt_flag;
    }
}
