// Note: For basic testing purposes, this is scanline-accurate. This should
// later be rewritten with cycle-accurate logic once we're past proof of concept
// and prototype stages.

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
            let mut sample = (self.duty >> self.sequence_counter) & 0b1;
            sample *= self.envelope.current_volume();
            return sample as i16;
        } else {
            return 0
        }
    }

    pub fn target_period(&mut self) -> u16 {
        let mut change_amount = self.period_initial >> self.sweep_shift;
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

    pub fn output(&self) -> i16 {
        if self.length_counter.length > 0 {
            let triangle_sequence = [15,14,13,12,11,10,9,8,7,6,5,4,3,2,1,0,
                                     0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15];
            return triangle_sequence[self.sequence_counter as usize];
        } else {
            return 0;
        }
    }
}

pub struct NoiseChannelState {
    pub length: u8,
    pub length_halt_flag: bool,

    pub envelope: VolumeEnvelopeState,
    pub length_counter: LengthCounterState,

    pub mode: u8,
    pub period: u16,

    // Actually a 15-bit register
    pub shift_register: u16,
}

impl NoiseChannelState {
    pub fn new() -> NoiseChannelState {
        return NoiseChannelState {
            length: 0,
            length_halt_flag: false,

            envelope: VolumeEnvelopeState::new(),
            length_counter: LengthCounterState::new(),
            mode: 0,
            period: 4068,

            // Actually a 15-bit register
            shift_register: 1,
        }
    }

    pub fn clock(&mut self) {
        let mut feedback = self.shift_register & 0b1;
        if self.mode == 1 {
            feedback ^= (self.shift_register >> 6) & 0b1;
        } else {
            feedback ^= (self.shift_register >> 1) & 0b1;
        }
        self.shift_register = self.shift_register >> 1;
        self.shift_register |= feedback << 14;
    }

    pub fn output(&self) -> i16 {
        if self.length_counter.length > 0 {
            let mut sample = (self.shift_register & 0b1) as u8;
            sample *= self.envelope.current_volume();
            return sample as i16;
        } else {
            return 0;
        }
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

    pub sample_buffer: [i16; 4096],
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
            sample_buffer: [0i16; 4096],
            sample_rate: 44100,
            cpu_clock_rate: 1_786_860,
            buffer_index: 0,
            generated_samples: 0,
            next_sample_at: 0,
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
                self.noise.period = noise_period[period_index as usize];
            },
            0x400F => {
                let length_index = (data & 0b1111_1000) >> 3;
                self.noise.length_counter.set_length(length_index);

                // Restart the envelope
                self.noise.envelope.start_flag = true;
            },

            // Status / Enabled
            0x4015 => {
                self.pulse_1.length_counter.channel_enabled  = (data & 0b0001) != 0;
                self.pulse_2.length_counter.channel_enabled  = (data & 0b0010) != 0;
                self.triangle.length_counter.channel_enabled = (data & 0b0100) != 0;
                self.noise.length_counter.channel_enabled    = (data & 0b1000) != 0;

                if ! (self.pulse_1.length_counter.channel_enabled) {
                    self.pulse_1.length_counter.length = 0;
                }
                if ! (self.pulse_2.length_counter.channel_enabled) {
                    self.pulse_2.length_counter.length = 0;
                }
                if ! (self.triangle.length_counter.channel_enabled) {
                    self.triangle.length_counter.length = 0;
                }
                if ! (self.noise.length_counter.channel_enabled) {
                    self.noise.length_counter.length = 0;
                }
            }

            // Frame Counter / Interrupts
            0x4017 => {
                self.frame_sequencer_mode = (data & 0b1000_0000) >> 7;
                self.disable_interrupt =    (data & 0b0100_0000) != 0;
                self.frame_reset_delay = 4;
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
                29828 => self.frame_interrupt = true,
                29829 => {
                    self.frame_interrupt = true;
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                },
                29830 => {
                    self.frame_interrupt = true;
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

    pub fn run_to_cycle(&mut self, target_cycle: u64) {
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
            }

            if self.current_cycle >= self.next_sample_at {
                // Mixing? Bah! Just throw the sample in the buffer.
                let mut composite_sample: i16 = 0;
                composite_sample += (self.pulse_1.output()  as i16 - 8) * 512; // Sure, why not?
                composite_sample += (self.pulse_2.output()  as i16 - 8) * 512;
                composite_sample += (self.triangle.output() as i16 - 8) * 512;
                composite_sample += (self.noise.output()    as i16 - 8) * 512;
                self.sample_buffer[self.buffer_index] = composite_sample;
                self.buffer_index = (self.buffer_index + 1) % self.sample_buffer.len();

                self.generated_samples += 1;
                self.next_sample_at = ((self.generated_samples + 1) * self.cpu_clock_rate) / self.sample_rate;

                if self.buffer_index == 0 {
                    self.dump_sample_buffer();
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
            buffer[i * 2]     = (((self.sample_buffer[i] as u16) & 0xFF00) >> 8) as u8;
            buffer[i * 2 + 1] = (((self.sample_buffer[i] as u16) & 0x00FF)     ) as u8;
        }

        file.write_all(&buffer);
    }
}
