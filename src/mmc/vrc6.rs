// Vrc6, 
// Reference capabilities: https://wiki.nesdev.com/w/index.php/VRC6

use ines::INesCartridge;
use memoryblock::MemoryBlock;

use mmc::mapper::*;
use mmc::mirroring;

pub struct Vrc6 {
    pub prg_rom: MemoryBlock,
    pub prg_ram: MemoryBlock,
    pub chr: MemoryBlock,
    pub vram: Vec<u8>,
    pub prg_ram_enable: bool,
    pub prg_bank_16: usize,
    pub prg_bank_8: usize,
    pub r: Vec<usize>,
    pub ppu_banking_mode: u8,
    pub mirroring_mode: u8,
    pub nametable_chrrom: bool,
    pub chr_a10_rules: bool,
    pub mirroring: Mirroring,

    pub irq_scanline_prescaler: u16,
    pub irq_latch: u8,
    pub irq_scanline_mode: bool,
    pub irq_enable: bool,
    pub irq_enable_after_acknowledgement: bool,
    pub irq_pending: bool,
    pub irq_counter: u8,
}

impl Vrc6 {
    pub fn from_ines(ines: INesCartridge) -> Result<Vrc6, String> {
        let prg_rom_block = ines.prg_rom_block();
        let prg_ram_block = ines.prg_ram_block()?;
        let chr_block = ines.chr_block()?;

        return Ok(Vrc6 {
            prg_rom: prg_rom_block.clone(),
            prg_ram: prg_ram_block.clone(),
            chr: chr_block.clone(),
            vram: vec![0u8; 0x1000],
            prg_ram_enable: false,
            prg_bank_16: 0,
            prg_bank_8: 0,
            r: vec![0usize; 8],
            ppu_banking_mode: 0,
            mirroring_mode: 0,
            nametable_chrrom: false,
            chr_a10_rules: false,
            mirroring: ines.header.mirroring(),

            irq_scanline_prescaler: 0,
            irq_latch: 0,
            irq_scanline_mode: false,
            irq_enable: false,
            irq_enable_after_acknowledgement: false,
            irq_pending: false,
            irq_counter: 0,
        });
    }

    fn _chr_mode_0(&self, address: u16) -> Option<u8> {
        // All 1k banks
        match address {
            0x0000 ..= 0x03FF => self.chr.banked_read(0x400, self.r[0], address as usize -  0x0000),
            0x0400 ..= 0x07FF => self.chr.banked_read(0x400, self.r[1], address as usize -  0x0400),
            0x0800 ..= 0x0BFF => self.chr.banked_read(0x400, self.r[2], address as usize -  0x0800),
            0x0C00 ..= 0x0FFF => self.chr.banked_read(0x400, self.r[3], address as usize -  0x0C00),
            0x1000 ..= 0x13FF => self.chr.banked_read(0x400, self.r[4], address as usize -  0x1000),
            0x1400 ..= 0x17FF => self.chr.banked_read(0x400, self.r[5], address as usize -  0x1400),
            0x1800 ..= 0x1BFF => self.chr.banked_read(0x400, self.r[6], address as usize -  0x1800),
            0x1C00 ..= 0x1FFF => self.chr.banked_read(0x400, self.r[7], address as usize -  0x1C00),
            _ => None // never reached
        }
    }

    fn _chr_mode_1(&self, address: u16) -> Option<u8> {
        // All 2k banks, with differing A10 behavior        
        if self.chr_a10_rules {
            //2k banks use PPU A10, ignore low bit of register
            match address {
                0x0000 ..= 0x07FF => self.chr.banked_read(0x800, (self.r[0] & 0xFE) >> 1, address as usize -  0x0000),
                0x0800 ..= 0x0FFF => self.chr.banked_read(0x800, (self.r[1] & 0xFE) >> 1, address as usize -  0x0800),
                0x1000 ..= 0x17FF => self.chr.banked_read(0x800, (self.r[2] & 0xFE) >> 1, address as usize -  0x1000),
                0x1800 ..= 0x1FFF => self.chr.banked_read(0x800, (self.r[3] & 0xFE) >> 1, address as usize -  0x1800),

                _ => None // never reached
            }
        } else {
            // Low bit of register determines A10, effectively duplicating 1k banks, similar to 1k mode
            match address {
                0x0000 ..= 0x03FF => self.chr.banked_read(0x400, self.r[0], address as usize -  0x0000),
                0x0400 ..= 0x07FF => self.chr.banked_read(0x400, self.r[0], address as usize -  0x0400),
                0x0800 ..= 0x0BFF => self.chr.banked_read(0x400, self.r[1], address as usize -  0x0800),
                0x0C00 ..= 0x0FFF => self.chr.banked_read(0x400, self.r[1], address as usize -  0x0C00),
                0x1000 ..= 0x13FF => self.chr.banked_read(0x400, self.r[2], address as usize -  0x1000),
                0x1400 ..= 0x17FF => self.chr.banked_read(0x400, self.r[2], address as usize -  0x1400),
                0x1800 ..= 0x1BFF => self.chr.banked_read(0x400, self.r[3], address as usize -  0x1800),
                0x1C00 ..= 0x1FFF => self.chr.banked_read(0x400, self.r[3], address as usize -  0x1C00),
                _ => None // never reached
            }
        }
    }

    fn _chr_mode_23(&self, address: u16) -> Option<u8> {
        // Essentially a mix, mode 0 for the upper half, with 2x 2k banks in the lower half that behave similarly to mode 1
        // but pull from R4-R5 instead
        match address {
            0x0000 ..= 0x0FFF => self._chr_mode_0(address),
            0x1000 ..= 0x1FFF => {
                if self.chr_a10_rules {
                    //2k banks use PPU A10, ignore low bit of register
                    match address {
                        0x1000 ..= 0x17FF => self.chr.banked_read(0x800, (self.r[4] & 0xFE) >> 1, address as usize -  0x1000),
                        0x1800 ..= 0x1FFF => self.chr.banked_read(0x800, (self.r[5] & 0xFE) >> 1, address as usize -  0x1800),
                        _ => None // never reached
                    }
                } else {
                    // Low bit of register determines A10, effectively duplicating 1k banks, similar to 1k mode
                    match address {
                        0x1000 ..= 0x13FF => self.chr.banked_read(0x400, self.r[4], address as usize -  0x1000),
                        0x1400 ..= 0x17FF => self.chr.banked_read(0x400, self.r[4], address as usize -  0x1400),
                        0x1800 ..= 0x1BFF => self.chr.banked_read(0x400, self.r[5], address as usize -  0x1800),
                        0x1C00 ..= 0x1FFF => self.chr.banked_read(0x400, self.r[5], address as usize -  0x1C00),
                        _ => None // never reached
                    }
                }
            }
            _ => None // never reached
        }
    }

    fn _mirroring_mode_0_read(&self, address: u16) -> Option<u8> {
        if self.nametable_chrrom {
            println!("Umimplemented CHR ROM nametables!");
            None
        } else {
            match self.mirroring_mode {
                0 => Some(self.vram[mirroring::vertical_mirroring(address) as usize]),
                1 => Some(self.vram[mirroring::horizontal_mirroring(address) as usize]),
                2 => Some(self.vram[mirroring::one_screen_lower(address) as usize]),
                3 => Some(self.vram[mirroring::one_screen_upper(address) as usize]),
                _ => None
            }
        }
    }

    fn _mirroring_mode_0_write(&mut self, address: u16, data: u8) {
        if self.nametable_chrrom {
            println!("Attempt to write to CHR ROM nametables!");
        } else {
            match self.mirroring_mode {
                0 => self.vram[mirroring::vertical_mirroring(address) as usize] = data,
                1 => self.vram[mirroring::horizontal_mirroring(address) as usize] = data,
                2 => self.vram[mirroring::one_screen_lower(address) as usize] = data,
                3 => self.vram[mirroring::one_screen_upper(address) as usize] = data,
                _ => {}
            }
        }
    }

    fn _clock_irq_prescaler(&mut self) {
        self.irq_scanline_prescaler += 3;
        if self.irq_scanline_prescaler >= 341 {
            self.irq_scanline_prescaler = 0;
            self._clock_irq_counter();
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

impl Mapper for Vrc6 {
    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
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

    fn debug_read_cpu(&self, address: u16) -> Option<u8> {
        match address {
            0x6000 ..= 0x7FFF => self.prg_ram.wrapping_read(address as usize - 0x6000),
            0x8000 ..= 0xBFFF => self.prg_rom.banked_read(0x4000, self.prg_bank_16, address as usize -  0x8000),
            0xC000 ..= 0xDFFF => self.prg_rom.banked_read(0x2000, self.prg_bank_8, address as usize -  0xC000),
            0xE000 ..= 0xFFFF => self.prg_rom.banked_read(0x2000, 0xFF, address as usize -  0xE000),
            _ => None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        match address {
            0x6000 ..= 0x7FFF => {
                if self.prg_ram_enable {
                    //self.prg_ram[(address - 0x6000) as usize] = data;
                    self.prg_ram.wrapping_write(address as usize - 0x6000, data)
                }
            },
            _ => {}
        }
        let masked_address = address & 0b1111_0000_0000_0011;
        match masked_address {
            0x8000 ..= 0x8003 => {
                self.prg_bank_16 = data as usize & 0x0F;
            },
            0xB003 => {
                self.ppu_banking_mode = data & 0b0000_0011;
                self.mirroring_mode = (data & 0b0000_1100) >> 2;
                self.nametable_chrrom = (data & 0b0001_0000) != 0;
                self.chr_a10_rules = (data & 0b0010_0000) != 0;
                self.prg_ram_enable = (data & 0b1000_0000) != 0;

                //println!("PPU Banking Mode: {}, CHR A10: {}", self.ppu_banking_mode, self.chr_a10_rules);
                //println!("Mirroring Mode: {}, Nametable CHR ROM: {}", self.mirroring_mode, self.nametable_chrrom);

            },
            0xC000 ..= 0xC003 => {
                self.prg_bank_8 = data as usize & 0x1F;
            },
            0xD000 => { self.r[0] = data as usize; },
            0xD001 => { self.r[1] = data as usize; },
            0xD002 => { self.r[2] = data as usize; },
            0xD003 => { self.r[3] = data as usize; },
            0xE000 => { self.r[4] = data as usize; },
            0xE001 => { self.r[5] = data as usize; },
            0xE002 => { self.r[6] = data as usize; },
            0xE003 => { self.r[7] = data as usize; },
            0xF000 => { self.irq_latch = data; },
            0xF001 => {
                self.irq_scanline_mode = ((data & 0b0000_0100) >> 2) == 0;
                self.irq_enable = (data & 0b0000_0010) != 0;
                self.irq_enable_after_acknowledgement = (data & 0b0000_0001) != 0;

                // acknowledge the pending IRQ if there is one
                self.irq_pending = false;

                // If the enable bit is set, setup for the next IRQ immediately, otherwise
                // do nothing (we may already have one in flight)
                if self.irq_enable {
                    self.irq_counter = self.irq_latch;
                    self.irq_scanline_prescaler = 0;
                }

            },
            0xF002 => {
                self.irq_pending = false;
                self.irq_enable = self.irq_enable_after_acknowledgement;
            }

            _ => {}
        }
    }

    fn debug_read_ppu(&self, address: u16) -> Option<u8> {
        match address {
            0x0000 ..= 0x1FFF => {
                // CHR Bank Selection
                match self.ppu_banking_mode {
                    0 => self._chr_mode_0(address),
                    1 => self._chr_mode_1(address),
                    2 => self._chr_mode_23(address),
                    3 => self._chr_mode_23(address),
                    _ => None
                }
            },
            0x2000 ..= 0x3FFF => {
                if self.chr_a10_rules {
                    match self.ppu_banking_mode {
                        0 => self._mirroring_mode_0_read(address),
                        //1 => self._mirroring_mode_1(address),
                        //2 => self._mirroring_mode_2(address),
                        //3 => self._mirroring_mode_3(address),
                        _ => {
                            println!("Unimplemented mirroring mode {}! Bailing.", self.ppu_banking_mode);
                            None
                        }
                    }
                } else {
                    // Unimplemented A10 weirdness
                    println!("Nametable CHROM in use! This is unimplemented, returning open bus.");
                    None
                }
            }
            _ => None
        }
    }

    fn write_ppu(&mut self, address: u16, data: u8) {
        match address {
            0x2000 ..= 0x3FFF => {
                if self.chr_a10_rules {
                    match self.ppu_banking_mode {
                        0 => self._mirroring_mode_0_write(address, data),
                        //1 => self._mirroring_mode_1(address),
                        //2 => self._mirroring_mode_2(address),
                        //3 => self._mirroring_mode_3(address),
                        _ => {
                            println!("Unimplemented mirroring mode {}! Bailing.", self.ppu_banking_mode);
                        }
                    }
                } else {
                    // Unimplemented A10 weirdness
                    println!("Nametable CHROM in use! This is unimplemented, returning open bus.");
                }
            }
            _ => {}
        }
    }
}