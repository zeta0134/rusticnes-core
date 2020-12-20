// Sunsoft FME-7, 5A, and 5B (notably lacking expansion audio for now)
// Reference implementation: https://wiki.nesdev.com/w/index.php/Sunsoft_FME-7

use cartridge::NesHeader;
use mmc::mapper::*;
use mmc::mirroring;

pub struct Fme7 {
    pub prg_rom: Vec<u8>,
    pub prg_ram: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub command: u8,
    pub chr_banks: Vec<usize>,
    pub prg_banks: Vec<usize>,
    pub prg_ram_enabled: bool,
    pub prg_ram_selected: bool,
    pub vram: Vec<u8>,
    pub mirroring: Mirroring,
    pub irq_enabled: bool,
    pub irq_counter_enabled: bool,
    pub irq_counter: u16,
    pub irq_pending: bool,
    pub audio_register_select: u8,
}

impl Fme7 {
    pub fn new(header: NesHeader, chr: &[u8], prg: &[u8]) -> Fme7 {
        return Fme7 {
            prg_rom: prg.to_vec(),
            chr_rom: chr.to_vec(),
            prg_ram: vec![0u8; header.prg_ram_size],
            command: 0,
            chr_banks: vec![0usize; 8],
            prg_banks: vec![0usize; 4],
            prg_ram_enabled: false,
            prg_ram_selected: false,
            vram: vec![0u8; 0x1000],
            mirroring: Mirroring::Vertical,
            irq_enabled: false,
            irq_counter_enabled: false,
            irq_counter: 0,
            irq_pending: false,
            audio_register_select: 0,
        }
    }

    pub fn clock_irq(&mut self) {
        if self.irq_counter_enabled {
            self.irq_counter = self.irq_counter.wrapping_sub(1);
            if self.irq_counter == 0xFFFF {
                self.irq_pending = true;
            }
        }
    }
}

impl Mapper for Fme7 {
    fn mirroring(&self) -> Mirroring {
        return Mirroring::Horizontal;
    }
    
    fn read_cpu(&mut self, address: u16) -> Option<u8> {
        let prg_rom_len = self.prg_rom.len();
        let prg_ram_len = self.prg_ram.len();
        match address {
            0x6000 ..= 0x7FFF => {
                if self.prg_ram_selected {
                    if self.prg_ram_enabled {
                        return Some(self.prg_ram[((self.prg_banks[0] * 0x2000) + (address as usize - 0x6000)) % prg_ram_len]);
                    } else {
                        return None
                    }
                } else {
                    return Some(self.prg_rom[((self.prg_banks[0] * 0x2000) + (address as usize - 0x6000)) % prg_ram_len]);
                }
            },
            0x8000 ..= 0x9FFF => return Some(self.prg_rom[((self.prg_banks[1] * 0x2000) + (address as usize - 0x8000)) % prg_rom_len]),
            0xA000 ..= 0xBFFF => return Some(self.prg_rom[((self.prg_banks[2] * 0x2000) + (address as usize - 0xA000)) % prg_rom_len]),
            0xC000 ..= 0xDFFF => return Some(self.prg_rom[((self.prg_banks[3] * 0x2000) + (address as usize - 0xC000)) % prg_rom_len]),
            0xE000 ..= 0xFFFF => return Some(self.prg_rom[(prg_rom_len - 0x2000) + (address as usize - 0xE000)]),
            _ => return None
        }
    }

    fn clock_cpu(&mut self) {
        self.clock_irq();
    }

    fn read_ppu(&mut self, address: u16) -> Option<u8> {
        let chr_rom_len = self.chr_rom.len();
        match address {
            0x0000 ..= 0x03FF => return Some(self.chr_rom[((self.chr_banks[0] * 0x400) + (address as usize - 0x0000)) % chr_rom_len]),
            0x0400 ..= 0x07FF => return Some(self.chr_rom[((self.chr_banks[1] * 0x400) + (address as usize - 0x0400)) % chr_rom_len]),
            0x0800 ..= 0x0BFF => return Some(self.chr_rom[((self.chr_banks[2] * 0x400) + (address as usize - 0x0800)) % chr_rom_len]),
            0x0C00 ..= 0x0FFF => return Some(self.chr_rom[((self.chr_banks[3] * 0x400) + (address as usize - 0x0C00)) % chr_rom_len]),
            0x1000 ..= 0x13FF => return Some(self.chr_rom[((self.chr_banks[4] * 0x400) + (address as usize - 0x1000)) % chr_rom_len]),
            0x1400 ..= 0x17FF => return Some(self.chr_rom[((self.chr_banks[5] * 0x400) + (address as usize - 0x1400)) % chr_rom_len]),
            0x1800 ..= 0x1BFF => return Some(self.chr_rom[((self.chr_banks[6] * 0x400) + (address as usize - 0x1800)) % chr_rom_len]),
            0x1C00 ..= 0x1FFF => return Some(self.chr_rom[((self.chr_banks[7] * 0x400) + (address as usize - 0x1C00)) % chr_rom_len]),
            0x2000 ..= 0x3FFF => return match self.mirroring {
                Mirroring::Horizontal => Some(self.vram[mirroring::horizontal_mirroring(address) as usize]),
                Mirroring::Vertical   => Some(self.vram[mirroring::vertical_mirroring(address) as usize]),
                Mirroring::OneScreenLower => Some(self.vram[mirroring::one_screen_lower(address) as usize]),
                Mirroring::OneScreenUpper => Some(self.vram[mirroring::one_screen_upper(address) as usize]),
                _ => None
            },
            _ => return None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        //self.clock_irq();
        match address {
            0x6000 ..= 0x7FFF => {
                if self.prg_ram_selected && self.prg_ram_enabled {
                    let prg_ram_len = self.prg_ram.len();
                    self.prg_ram[((self.prg_banks[0] * 0x2000) + (address as usize - 0x6000)) % prg_ram_len] = data;
                }
            },
            0x8000 ..= 0x9FFF => {
                // Store the command to execute next
                self.command = data & 0b0000_1111;
            },
            0xA000 ..= 0xBFFF => {
                // Execute the stored command with the provided parameter byte
                match self.command {
                    0x0 ..= 0x7 => { self.chr_banks[self.command as usize] = data as usize},
                    0x8 =>  {
                        self.prg_ram_enabled = (data & 0b1000_0000) != 0;
                        self.prg_ram_selected = (data & 0b0100_0000) != 0;
                        self.prg_banks[0] = (data & 0b0011_1111) as usize;
                    },
                    0x9 ..= 0xB => {
                        self.prg_banks[(self.command - 0x8) as usize] = (data & 0b0011_1111) as usize;
                    },
                    0xC => {
                        match data & 0b0000_0011 {
                            0 => self.mirroring = Mirroring::Vertical,
                            1 => self.mirroring = Mirroring::Horizontal,
                            2 => self.mirroring = Mirroring::OneScreenLower,
                            3 => self.mirroring = Mirroring::OneScreenUpper,
                            _ => {}
                        }
                    },
                    0xD => {
                        // writes to this register always acknowledge any pending IRQ
                        self.irq_pending = false;
                        self.irq_enabled = (data & 0b0000_0001) != 0;
                        self.irq_counter_enabled = (data & 0b1000_0000) != 0;
                    },
                    0xE => {
                        self.irq_counter = (self.irq_counter & 0xFF00) + (data as u16);
                    },
                    0xF => {
                        self.irq_counter = (self.irq_counter & 0x00FF) + ((data as u16) << 8);
                    },
                    _ => {}
                }
            },
            0xC000 ..= 0xDFFF => {
                self.audio_register_select = (data & 0x0F);
            },

            _ => {}
        }
    }

    fn write_ppu(&mut self, address: u16, data: u8) {
        match address {
            0x2000 ..= 0x3FFF => match self.mirroring {
                Mirroring::Horizontal => self.vram[mirroring::horizontal_mirroring(address) as usize] = data,
                Mirroring::Vertical   => self.vram[mirroring::vertical_mirroring(address) as usize] = data,
                Mirroring::OneScreenLower => self.vram[mirroring::one_screen_lower(address) as usize] = data,
                Mirroring::OneScreenUpper => self.vram[mirroring::one_screen_upper(address) as usize] = data,
                _ => {}
            },
            _ => {}
        }
    }

    fn irq_flag(&self) -> bool {
        return self.irq_enabled && self.irq_pending;
    }
}

struct ToneGenerator {
    pub period_compare: u16,
    pub period_current: u16,
    pub output: u8,
}

impl ToneGenerator {
    pub fn new() -> ToneGenerator {
        return ToneGenerator {
            period_compare: 0,
            period_current: 0,
            output: 0,
        }
    }

    pub fn clock(&mut self) {
        self.period_current += 1;
        if self.period_current >= self.period_compare {
            self.period_current = 0;
            self.output = self.output & 0b1;
        }
    }

    pub fn output(&self) -> u8 {
        return self.output;
    }
}

struct NoiseGenerator {
    pub period_compare: u16,
    pub period_current: u16,
    // Actually a 17bit register, higher bits are unused
    pub shift_register: u32,
}

impl NoiseGenerator {
    pub fn new() -> NoiseGenerator {
        return NoiseGenerator {
            period_compare: 0,
            period_current: 0,
            shift_register: 0,
        }
    }

    pub fn advance_lfsr(&mut self) {
        let output = self.shift_register & 0b1;
        let tap13 = (self.shift_register & 0b0000_0010_0000_0000_0000) >> 13;
        let tap16 = (self.shift_register & 0b0001_0000_0000_0000_0000) >> 16;
        let new_bit_17 = output ^ tap13 ^ tap16;
        self.shift_register = self.shift_register >> 1;
        self.shift_register += new_bit_17 << 17
    }

    pub fn clock(&mut self) {
        self.period_current += 1;
        if self.period_current >= self.period_compare {
            self.period_current = 0;
            self.advance_lfsr();
        }
    }

    pub fn output(&self) -> u8 {
        return (self.shift_register & 0b1) as u8;
    }
}

struct EnvelopeGenerator {
    pub period_compare: u16,
    pub period_current: u16,
    pub continue_flag: bool,
    pub attack_flag: bool,
    pub alternate_flag: bool,
    pub hold_flag: bool,
    pub current_value: u8,
    pub increasing: bool,
    pub holding: bool,
}

impl EnvelopeGenerator {
    pub fn new() -> EnvelopeGenerator {
        return EnvelopeGenerator {
            period_compare: 0,
            period_current: 0,
            continue_flag: false,
            attack_flag: false,
            alternate_flag: false,
            hold_flag: false,
            current_value: 0,
            increasing: false,
            holding: false,
        }
    }

    pub fn restart_envelope(&mut self) {
        self.holding = false;
        if self.attack_flag {
            self.increasing = true;
            self.current_value = 0;
        } else {
            self.increasing = false;
            self.current_value = 31;
        }
    }

    pub fn advance_envelope(&mut self) {
        if self.holding {
            return;
        }

        if self.increasing {
            self.current_value += 1;
        } else {
            self.current_value -= 1;
        }

        if (self.current_value == 0) || (self.current_value == 32) {
            // We've reached a boundary; decide how to proceed
            if !(self.continue_flag) {
                // non-continue mode, choose a value to hold
                // and exit immediately. Note an oddity here,
                // we *always* hold the value 0 in non-continue
                // mode.
                self.current_value = 0;
                self.holding = true;
            } else {
                if self.hold_flag {
                    // Hold this value, with an optional flip first
                    // (this is the only way to get the more intuitive
                    // "increase and hold" behavior)
                    self.holding = true;
                    if self.alternate_flag {
                        if self.attack_flag {
                            self.current_value = 0;
                        } else {
                            self.current_value = 31;
                        }
                    }
                }

                // Deal with switching directions, and fix the 5-bit overflow
                if self.alternate_flag {
                    if self.increasing {
                        self.current_value = 31;
                    }
                    self.increasing = !(self.increasing);
                } else {
                    if self.increasing {
                        self.current_value = 0;
                    } else {
                        self.current_value = 31;
                    }
                }
            }
        }
    }

    pub fn clock(&mut self) {
        self.period_current += 1;
        if self.period_current >= self.period_compare {
            self.period_current = 0;
            self.advance_envelope();
        }
    }
}

struct YmChannel {
    pub tone: ToneGenerator,
    pub tone_enabled: bool,
    pub noise_enabled: bool,
    pub envelope_enabled: bool,
    pub static_volume: u8
}

struct YM2149F {
    pub channel_a: YmChannel,
    pub channel_b: YmChannel,
    pub channel_c: YmChannel,
    pub noise: NoiseGenerator,
    pub envelope: EnvelopeGenerator,
    pub clock_divider_counter: u8,
}

