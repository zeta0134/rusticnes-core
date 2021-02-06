use mmc::mapper::Mapper;

pub struct DmcState {
    pub debug_disable: bool,
    pub debug_buffer: Vec<i16>,

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
    pub rdy_line: bool,
    pub rdy_delay: u8,
}

impl DmcState {
    pub fn new() -> DmcState {
        return DmcState {
            debug_disable: false,
            debug_buffer: vec!(0i16; 4096),

            looping: false,
            period_initial: 428,
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
            rdy_line: false,
            rdy_delay: 0,
        }
    }

    pub fn debug_status(&self) -> String {
        return format!("Rate: {:3} - Divisor: {:3} - Start: {:04X} - Current: {:04X} - Length: {:4} - R.Bytes: {:4} - R.Bits: {:1}", 
            self.period_initial, self.period_current, self.starting_address, self.current_address, self.sample_length,
            self.bytes_remaining, self.bits_remaining);
    }

    pub fn read_next_sample(&mut self, mapper: &mut dyn Mapper) {
        match mapper.read_cpu(0x8000 | (self.current_address & 0x7FFF)) {
            Some(byte) => self.sample_buffer = byte,
            None => self.sample_buffer = 0,
        }
        self.current_address = self.current_address.wrapping_add(1);
        self.bytes_remaining -= 1;
        if self.bytes_remaining == 0 {
            if self.looping {
                self.current_address = self.starting_address;
                self.bytes_remaining = self.sample_length;
            } else {
                if self.interrupt_enabled {
                    self.interrupt_flag = true;
                }
            }
        }
        self.sample_buffer_empty = false;
        self.rdy_line = false;
        self.rdy_delay = 0;
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

    pub fn clock(&mut self, mapper: &mut dyn Mapper) {
        if self.period_current == 0 {
            self.period_current = self.period_initial - 1;
            self.update_output_unit();
        } else {
            self.period_current -= 1;
        }
        if self.sample_buffer_empty && self.bytes_remaining > 0 {
            self.rdy_line = true;
            self.rdy_delay += 1;
            if self.rdy_delay > 2 {
                self.read_next_sample(mapper);
            }
        } else {
            self.rdy_line = false;
            self.rdy_delay = 0;
        }
    }

    pub fn output(&self) -> i16 {
        return self.output_level as i16;
    }
}