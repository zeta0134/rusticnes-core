// https://www.nesdev.org/wiki/VRC7
// https://www.nesdev.org/wiki/VRC7_audio

use ines::INesCartridge;
use memoryblock::MemoryBlock;

use mmc::mapper::*;
use mmc::mirroring;

pub struct Vrc7 {
    pub prg_rom: MemoryBlock,
    pub prg_ram: MemoryBlock,
    pub chr: MemoryBlock,

    pub mirroring: Mirroring,
    pub vram: Vec<u8>,

    pub chr_banks: Vec<u8>,
    pub prg_banks: Vec<u8>,
    pub submapper: u8,

    pub irq_scanline_prescaler: i16,
    pub irq_latch: u8,
    pub irq_scanline_mode: bool,
    pub irq_enable: bool,
    pub irq_enable_after_acknowledgement: bool,
    pub irq_pending: bool,
    pub irq_counter: u8,

    pub audio: Vrc7Audio,
}

impl Vrc7 {
    pub fn from_ines(ines: INesCartridge) -> Result<Vrc7, String> {
        let prg_rom_block = ines.prg_rom_block();
        let prg_ram_block = ines.prg_ram_block()?;
        let chr_block = ines.chr_block()?;

        return Ok(Vrc7 {
            prg_rom: prg_rom_block.clone(),
            prg_ram: prg_ram_block.clone(),
            chr: chr_block.clone(),
            mirroring: ines.header.mirroring(),
            vram: vec![0u8; 0x1000],
            chr_banks: vec![0u8; 8],
            prg_banks: vec![0u8; 3],
            submapper: ines.header.submapper_number(),
            
            irq_scanline_prescaler: 0,
            irq_latch: 0,
            irq_scanline_mode: false,
            irq_enable: false,
            irq_enable_after_acknowledgement: false,
            irq_pending: false,
            irq_counter: 0,

            audio: Vrc7Audio::new(),
        });
    }

    fn _clock_irq_prescaler(&mut self) {
        self.irq_scanline_prescaler -= 3;
        if self.irq_scanline_prescaler <= 0 {
            self._clock_irq_counter();
            self.irq_scanline_prescaler += 341;
        }
    }

    fn _clock_irq_counter(&mut self) {
        if self.irq_counter == 0xFF {
            self.irq_counter = self.irq_latch;
            self.irq_pending = true;
        } else {
            self.irq_counter += 1;
        }
    }
}

impl Mapper for Vrc7 {
    fn print_debug_status(&self) {
        println!("======= VRC7 =======");
        println!("Mirroring Mode: {}", mirroring_mode_name(self.mirroring));
        println!("====================");
    }

    fn clock_cpu(&mut self) {
        if self.irq_enable {
            if self.irq_scanline_mode {
                self._clock_irq_prescaler();
            } else {
                self._clock_irq_counter();
            }
        }
    }

    fn irq_flag(&self) -> bool {
        return self.irq_pending;
    }

    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }
    
    fn debug_read_cpu(&self, address: u16) -> Option<u8> {
        match address {
            0x6000 ..= 0x7FFF => {self.prg_ram.wrapping_read((address - 0x6000) as usize)},
            0x8000 ..= 0x9FFF => self.prg_rom.banked_read(0x2000, self.prg_banks[0] as usize, address as usize),
            0xA000 ..= 0xBFFF => self.prg_rom.banked_read(0x2000, self.prg_banks[1] as usize, address as usize),
            0xC000 ..= 0xDFFF => self.prg_rom.banked_read(0x2000, self.prg_banks[2] as usize, address as usize),
            0xE000 ..= 0xFFFF => self.prg_rom.banked_read(0x2000, 0xFF, address as usize),
            _ => None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        match address {
            0x6000 ..= 0x7FFF => {self.prg_ram.wrapping_write((address - 0x6000) as usize, data);},
            0x8000 ..= 0xFFFF => {
                let register_mask = match self.submapper {
                    1 => 0xF008,
                    2 => 0xF010,
                    _ => 0xF010
                };
                let register_address = address & register_mask;
                match register_address {
                    0x8000          => {self.prg_banks[0] = data & 0b0011_1111},
                    0x8010 | 0x8008 => {self.prg_banks[1] = data & 0b0011_1111},
                    0x9000          => {self.prg_banks[2] = data & 0b0011_1111},
                    0xA000          => {self.chr_banks[0] = data},
                    0xA008 | 0xA010 => {self.chr_banks[1] = data},
                    0xB000          => {self.chr_banks[2] = data},
                    0xB008 | 0xB010 => {self.chr_banks[3] = data},
                    0xC000          => {self.chr_banks[4] = data},
                    0xC008 | 0xC010 => {self.chr_banks[5] = data},
                    0xD000          => {self.chr_banks[6] = data},
                    0xD008 | 0xD010 => {self.chr_banks[7] = data},
                    // Sound would go here!!
                    0xE000         => {
                        match data & 0b0000_0011 {
                            0 => self.mirroring = Mirroring::Vertical,
                            1 => self.mirroring = Mirroring::Horizontal,
                            2 => self.mirroring = Mirroring::OneScreenLower,
                            3 => self.mirroring = Mirroring::OneScreenUpper,
                            _ => {}
                        }
                        // for now, ignoring both WRAM protect and sound reset
                    },
                    0xE008 | 0xE010 => { self.irq_latch = data; },
                    0xF000         => {
                        self.irq_scanline_mode = ((data & 0b0000_0100) >> 2) == 0;
                        self.irq_enable = (data & 0b0000_0010) != 0;
                        self.irq_enable_after_acknowledgement = (data & 0b0000_0001) != 0;

                        // acknowledge the pending IRQ if there is one
                        self.irq_pending = false;

                        // If the enable bit is set, setup for the next IRQ immediately, otherwise
                        // do nothing (we may already have one in flight)
                        if self.irq_enable {
                            self.irq_counter = self.irq_latch;
                            self.irq_scanline_prescaler = 341;                    
                        }

                    },
                    0xF008 | 0xF010 => {
                        self.irq_pending = false;
                        self.irq_enable = self.irq_enable_after_acknowledgement;
                    },
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn debug_read_ppu(&self, address: u16) -> Option<u8> {
        match address {
            0x0000 ..= 0x03FF => {self.chr.banked_read(0x400, self.chr_banks[0] as usize, address as usize)},
            0x0400 ..= 0x07FF => {self.chr.banked_read(0x400, self.chr_banks[1] as usize, address as usize)},
            0x0800 ..= 0x0BFF => {self.chr.banked_read(0x400, self.chr_banks[2] as usize, address as usize)},
            0x0C00 ..= 0x0FFF => {self.chr.banked_read(0x400, self.chr_banks[3] as usize, address as usize)},
            0x1000 ..= 0x13FF => {self.chr.banked_read(0x400, self.chr_banks[4] as usize, address as usize)},
            0x1400 ..= 0x17FF => {self.chr.banked_read(0x400, self.chr_banks[5] as usize, address as usize)},
            0x1800 ..= 0x1BFF => {self.chr.banked_read(0x400, self.chr_banks[6] as usize, address as usize)},
            0x1C00 ..= 0x1FFF => {self.chr.banked_read(0x400, self.chr_banks[7] as usize, address as usize)},
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

    fn write_ppu(&mut self, address: u16, data: u8) {
        match address {
            0x0000 ..= 0x03FF => {self.chr.banked_write(0x400, self.chr_banks[0] as usize, address as usize, data)},
            0x0400 ..= 0x07FF => {self.chr.banked_write(0x400, self.chr_banks[1] as usize, address as usize, data)},
            0x0800 ..= 0x0BFF => {self.chr.banked_write(0x400, self.chr_banks[2] as usize, address as usize, data)},
            0x0C00 ..= 0x0FFF => {self.chr.banked_write(0x400, self.chr_banks[3] as usize, address as usize, data)},
            0x1000 ..= 0x13FF => {self.chr.banked_write(0x400, self.chr_banks[4] as usize, address as usize, data)},
            0x1400 ..= 0x17FF => {self.chr.banked_write(0x400, self.chr_banks[5] as usize, address as usize, data)},
            0x1800 ..= 0x1BFF => {self.chr.banked_write(0x400, self.chr_banks[6] as usize, address as usize, data)},
            0x1C00 ..= 0x1FFF => {self.chr.banked_write(0x400, self.chr_banks[7] as usize, address as usize, data)},
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
}

// TODO: explore and see if we can't somehow make these constant while keeping them
// in function form. (We ideally do not want to store the full baked table in source)
fn generate_logsin_lut() -> Vec<u16> {
    let mut logsin_lut = vec!(0u16; 256);
    for n in 0 ..= 255 {
        let i = n as f32 + 0.5;
        let x = i * (std::f32::consts::PI / 2.0) / 256.0;
        logsin_lut[n] = (f32::log2(f32::sin(x)) * -256.0) as u16;
    }
    return logsin_lut;
}

fn generate_exp_table() -> Vec<u16> {
    let mut exp_lut = vec!(0u16; 256);
    for n in 0 ..= 255 {
        let i = n as f32 / 256.0;
        exp_lut[n] = ((f32::exp2(i) * 1024.0) - 1024.0) as u16
    }
    return exp_lut;
}

pub struct Vrc7AudioChannel {
    fnum: u32,
    octave: u32,
    carrier_phase: u32,
    volume: u8,
    logsin_lut: Vec<u16>,
    exp_lut: Vec<u16>,
}

impl Vrc7AudioChannel {
    pub fn new() -> Vrc7AudioChannel {
        return Vrc7AudioChannel {
            fnum: 0,
            octave: 0,
            carrier_phase: 0,
            volume: 0,
            logsin_lut: generate_logsin_lut(),
            exp_lut: generate_exp_table(),
        };
    }

    pub fn lookup_logsin(&self, i: usize) -> u16 {
        let quadrant = (i & 0x300) >> 8;
        let index = i & 0xFF;
        match  quadrant {
            0 => self.logsin_lut[index],
            1 => self.logsin_lut[255 - index],
            2 => 0x8000 | self.logsin_lut[index],
            3 => 0x8000 | self.logsin_lut[255 - index],
            _ => {0} // should be unreachable
        }
    }

    pub fn lookup_exp(&self, i: u16) -> i16 {
        let sign = i & 0x8000;
        let integral_magnitude =    (i & 0x7F00) >> 8;
        let fractional_magnitude =   i & 0x00FF;
        let t_value = (self.exp_lut[(255 - fractional_magnitude) as usize] + 1024) << 1;
        let mut result = (t_value >> integral_magnitude) as i16;
        if sign != 0 {
            result = !result;
        }
        return result >> 4;
    }

    pub fn adjusted_sine(&self, phase: usize, volume: u16, eg_level: u16) -> i16 {
        return self.lookup_exp(self.lookup_logsin(phase) + 128 * volume + 16 * eg_level);
    }

    pub fn update(&mut self) {

    }

    pub fn output(&self) -> i16 {
        // TODO
        return 0;
    }
}

pub struct Vrc7Audio {
    pub channel1: Vrc7AudioChannel,
    pub channel2: Vrc7AudioChannel,
    pub channel3: Vrc7AudioChannel,
    pub channel4: Vrc7AudioChannel,
    pub channel5: Vrc7AudioChannel,
    pub channel6: Vrc7AudioChannel,
}

impl Vrc7Audio {
    pub fn new() -> Vrc7Audio {
        return Vrc7Audio {
            channel1: Vrc7AudioChannel::new(),
            channel2: Vrc7AudioChannel::new(),
            channel3: Vrc7AudioChannel::new(),
            channel4: Vrc7AudioChannel::new(),
            channel5: Vrc7AudioChannel::new(),
            channel6: Vrc7AudioChannel::new(),
        };
    }

    pub fn clock() {

    }
}
