// Advanced mapper with bank-switched PRG ROM and CHR ROM, and a scanline counter feeding into IRQ
// Reference capabilities: https://wiki.nesdev.com/w/index.php/MMC3

use cartridge::NesHeader;
use mmc::mapper::*;

pub struct Mmc3 {
    pub prg_rom: Vec<u8>,
    pub prg_ram: Vec<u8>,
    pub chr_rom: Vec<u8>,

    pub chr2_bank_0: usize,
    pub chr2_bank_1: usize,
    pub chr1_bank_2: usize,
    pub chr1_bank_3: usize,
    pub chr1_bank_4: usize,
    pub chr1_bank_5: usize,
    pub chr_ram: bool,

    pub prg_bank_6: usize,
    pub prg_bank_7: usize,

    pub switch_chr_banks: bool,
    pub switch_prg_banks: bool,

    pub bank_select: u8,

    pub irq_counter: u8,
    pub irq_reload: u8,
    pub irq_reload_requested: bool,
    pub irq_enabled: bool,
    pub irq_flag: bool,

    pub last_a12: u8,

    // Debug
    pub last_chr_read: u16,

    pub mirroring: Mirroring,
}

impl Mmc3 {
    pub fn new(header: NesHeader, chr: &[u8], prg: &[u8]) -> Mmc3 {
        let chr_rom = match header.has_chr_ram {
            true => vec![0u8; 8 * 1024],
            false => chr.to_vec()
        };

        return Mmc3 {
            prg_rom: prg.to_vec(),
            prg_ram: vec![0u8; 0x2000],
            chr_rom: chr_rom,
            // Note: On real MMC3-based hardware, many of these values are random on startup, so
            // the defaults presented below are arbitrary.
            chr2_bank_0: 0,
            chr2_bank_1: 0,
            chr1_bank_2: 0,
            chr1_bank_3: 0,
            chr1_bank_4: 0,
            chr1_bank_5: 0,
            chr_ram: header.has_chr_ram,

            prg_bank_6: 0,
            prg_bank_7: 0,

            switch_chr_banks: false,
            switch_prg_banks: false,

            bank_select: 0,

            irq_counter: 0,
            irq_reload: 0,
            irq_reload_requested: false,
            irq_enabled: false,
            irq_flag: false,

            last_a12: 0,
            last_chr_read: 0,

            mirroring: Mirroring::Vertical,
        }
    }

    fn _read_byte(&mut self, address: u16, side_effects: bool) -> Option<u8> {
        match address {
            // CHR
            0x0000 ... 0x1FFF => {
                if side_effects {
                    self.last_chr_read = address;
                    let current_a12 = ((address & 0b0001_0000_0000_0000) >> 12) as u8;
                    if current_a12 == 1 && self.last_a12 == 0 {
                        if self.irq_counter == 0 || self.irq_reload_requested {
                            self.irq_counter = self.irq_reload;
                            self.irq_reload_requested = false;
                        } else {
                            self.irq_counter -= 1;                        
                        }
                        if self.irq_counter == 0 && self.irq_enabled {
                            self.irq_flag = true;                        
                        }
                    }
                    self.last_a12 = current_a12;
                }
                let chr_rom_len = self.chr_rom.len();
                if self.switch_chr_banks {
                    match address {
                        0x0000 ... 0x03FF => return Some(self.chr_rom[((self.chr1_bank_2 * 0x400) + (address as usize -  0x000)) % chr_rom_len]),
                        0x0400 ... 0x07FF => return Some(self.chr_rom[((self.chr1_bank_3 * 0x400) + (address as usize -  0x400)) % chr_rom_len]),
                        0x0800 ... 0x0BFF => return Some(self.chr_rom[((self.chr1_bank_4 * 0x400) + (address as usize -  0x800)) % chr_rom_len]),
                        0x0C00 ... 0x0FFF => return Some(self.chr_rom[((self.chr1_bank_5 * 0x400) + (address as usize -  0xC00)) % chr_rom_len]),
                        0x1000 ... 0x17FF => return Some(self.chr_rom[((self.chr2_bank_0 * 0x400) + (address as usize - 0x1000)) % chr_rom_len]),
                        0x1800 ... 0x1FFF => return Some(self.chr_rom[((self.chr2_bank_1 * 0x400) + (address as usize - 0x1800)) % chr_rom_len]),
                        _ => return None,
                    }
                } else {
                    match address {
                        0x0000 ... 0x07FF => return Some(self.chr_rom[((self.chr2_bank_0 * 0x400) + (address as usize -  0x000)) % chr_rom_len]),
                        0x0800 ... 0x0FFF => return Some(self.chr_rom[((self.chr2_bank_1 * 0x400) + (address as usize -  0x800)) % chr_rom_len]),
                        0x1000 ... 0x13FF => return Some(self.chr_rom[((self.chr1_bank_2 * 0x400) + (address as usize - 0x1000)) % chr_rom_len]),
                        0x1400 ... 0x17FF => return Some(self.chr_rom[((self.chr1_bank_3 * 0x400) + (address as usize - 0x1400)) % chr_rom_len]),
                        0x1800 ... 0x1BFF => return Some(self.chr_rom[((self.chr1_bank_4 * 0x400) + (address as usize - 0x1800)) % chr_rom_len]),
                        0x1C00 ... 0x1FFF => return Some(self.chr_rom[((self.chr1_bank_5 * 0x400) + (address as usize - 0x1C00)) % chr_rom_len]),
                        _ => return None,
                    }
                }
            },
            // PRG RAM
            0x6000 ... 0x7FFF => {
                return Some(self.prg_ram[(address - 0x6000) as usize]);
            },
            // PRG ROM
            0x8000 ... 0xFFFF => {
                let prg_rom_len = self.prg_rom.len();
                if self.switch_prg_banks {
                    match address {
                        0x8000 ... 0x9FFF => return Some(self.prg_rom[((self.prg_rom.len() - 0x4000) + (address as usize -  0x8000)) % prg_rom_len]),
                        0xA000 ... 0xBFFF => return Some(self.prg_rom[((self.prg_bank_7 * 0x2000)    + (address as usize -  0xA000)) % prg_rom_len]),
                        0xC000 ... 0xDFFF => return Some(self.prg_rom[((self.prg_bank_6 * 0x2000)    + (address as usize -  0xC000)) % prg_rom_len]),
                        0xE000 ... 0xFFFF => return Some(self.prg_rom[((self.prg_rom.len() - 0x2000) + (address as usize -  0xE000)) % prg_rom_len]),
                        _ => return None,
                    }
                } else {
                    match address {
                        0x8000 ... 0x9FFF => return Some(self.prg_rom[((self.prg_bank_6 * 0x2000)    + (address as usize -  0x8000)) % prg_rom_len]),
                        0xA000 ... 0xBFFF => return Some(self.prg_rom[((self.prg_bank_7 * 0x2000)    + (address as usize -  0xA000)) % prg_rom_len]),
                        0xC000 ... 0xDFFF => return Some(self.prg_rom[((self.prg_rom.len() - 0x4000) + (address as usize -  0xC000)) % prg_rom_len]),
                        0xE000 ... 0xFFFF => return Some(self.prg_rom[((self.prg_rom.len() - 0x2000) + (address as usize -  0xE000)) % prg_rom_len]),
                        _ => return None,
                    }
                }
            },
            _ => return None
        }
    }
}

impl Mapper for Mmc3 {
    fn print_debug_status(&self) {
        println!("======= MMC3 =======");
        println!("IRQ: Current: {}, Reload: {}", self.irq_counter, self.irq_reload);
        println!("Last A12: {}, Last CHR Read: 0x{:04X}", self.last_a12, self.last_chr_read);
        println!("====================");
    }

    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }

    fn irq_flag(&self) -> bool {
        return self.irq_flag;
    }

    fn read_byte(&mut self, address: u16) -> Option<u8> {
        return self._read_byte(address, true);
    }

    fn debug_read_byte(&mut self, address: u16) -> Option<u8> {
        return self._read_byte(address, false);
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            // CHR RAM (if enabled)
            0x0000 ... 0x1FFF => {
                self.last_chr_read = address;
                let current_a12 = ((address & 0b0001_0000_0000_0000) >> 12) as u8;
                if current_a12 == 1 && self.last_a12 == 0 {
                    if self.irq_counter == 0 || self.irq_reload_requested {
                        self.irq_counter = self.irq_reload;
                        self.irq_reload_requested = false;
                    } else {
                        self.irq_counter -= 1;                        
                    }
                    if self.irq_counter == 0 && self.irq_enabled {
                        self.irq_flag = true;                        
                    }
                }
                self.last_a12 = current_a12;
                if self.chr_ram {
                    let chr_rom_len = self.chr_rom.len();
                    if self.switch_chr_banks {
                        match address {
                            0x0000 ... 0x03FF => self.chr_rom[((self.chr1_bank_2 * 0x400) + (address as usize -  0x000)) % chr_rom_len] = data,
                            0x0400 ... 0x07FF => self.chr_rom[((self.chr1_bank_3 * 0x400) + (address as usize -  0x400)) % chr_rom_len] = data,
                            0x0800 ... 0x0BFF => self.chr_rom[((self.chr1_bank_4 * 0x400) + (address as usize -  0x800)) % chr_rom_len] = data,
                            0x0C00 ... 0x0FFF => self.chr_rom[((self.chr1_bank_5 * 0x400) + (address as usize -  0xC00)) % chr_rom_len] = data,
                            0x1000 ... 0x17FF => self.chr_rom[((self.chr2_bank_0 * 0x400) + (address as usize - 0x1000)) % chr_rom_len] = data,
                            0x1800 ... 0x1FFF => self.chr_rom[((self.chr2_bank_1 * 0x400) + (address as usize - 0x1800)) % chr_rom_len] = data,
                            _ => (),
                        }
                    } else {
                        match address {
                            0x0000 ... 0x07FF => self.chr_rom[((self.chr2_bank_0 * 0x400) + (address as usize -  0x000)) % chr_rom_len] = data,
                            0x0800 ... 0x0FFF => self.chr_rom[((self.chr2_bank_1 * 0x400) + (address as usize -  0x800)) % chr_rom_len] = data,
                            0x1000 ... 0x13FF => self.chr_rom[((self.chr1_bank_2 * 0x400) + (address as usize - 0x1000)) % chr_rom_len] = data,
                            0x1400 ... 0x17FF => self.chr_rom[((self.chr1_bank_3 * 0x400) + (address as usize - 0x1400)) % chr_rom_len] = data,
                            0x1800 ... 0x1BFF => self.chr_rom[((self.chr1_bank_4 * 0x400) + (address as usize - 0x1800)) % chr_rom_len] = data,
                            0x1C00 ... 0x1FFF => self.chr_rom[((self.chr1_bank_5 * 0x400) + (address as usize - 0x1C00)) % chr_rom_len] = data,
                            _ => (),
                        }
                    }
                }
            },
            // PRG RAM
            0x6000 ... 0x7FFF => {
                // Note: Intentionally omitting PRG RAM protection feature, since this
                // retains compatability with assumptions about iNES mapper 004
                self.prg_ram[address as usize - 0x6000] = data;
            },
            // Registers
            0x8000 ... 0xFFFF => {
                if address & 0b1 == 0 {
                    // Even Registers
                    match address {
                        0x8000 ... 0x9FFF => {
                            // Bank Select
                            self.bank_select =      data & 0b0000_0111;
                            self.switch_prg_banks = (data & 0b0100_0000) != 0;
                            self.switch_chr_banks = (data & 0b1000_0000) != 0;
                        },
                        0xA000 ... 0xBFFF => {
                            if data & 0b1 == 0 {
                                self.mirroring = Mirroring::Vertical;
                            } else {
                                self.mirroring = Mirroring::Horizontal;
                            }
                        },
                        0xC000 ... 0xDFFF => {
                            self.irq_reload = data;
                        },
                        0xE000 ... 0xFFFF => {
                            self.irq_enabled = false;
                            self.irq_flag = false;
                        }

                        _ => (),
                    }
                } else {
                    // Odd Registers
                    match address {
                        0x8000 ... 0x9FFF => {
                            // Bank Data
                            match self.bank_select {
                                0 => self.chr2_bank_0 = (data & 0b1111_1110) as usize,
                                1 => self.chr2_bank_1 = (data & 0b1111_1110) as usize,
                                2 => self.chr1_bank_2 = data as usize,
                                3 => self.chr1_bank_3 = data as usize,
                                4 => self.chr1_bank_4 = data as usize,
                                5 => self.chr1_bank_5 = data as usize,
                                6 => self.prg_bank_6  = (data & 0b0011_1111) as usize,
                                7 => self.prg_bank_7  = (data & 0b0011_1111) as usize,
                                _ => (),
                            }
                        },
                        0xA000 ... 0xBFFF => {
                            // PRG RAM Protect
                            // Intentionally not emulated, for compatability with iNES mapper 004
                        },
                        0xC000 ... 0xDFFF => {
                            self.irq_reload_requested = true;
                        },
                        0xE000 ... 0xFFFF => {
                            self.irq_enabled = true;
                        }
                        _ => (),
                    }
                }
            },
            _ => (),
        }
    }
    
    fn has_sram(&self) -> bool {
        return true;
    }

    fn get_sram(&self) -> Vec<u8> {
        return self.prg_ram.to_vec();
    }

    fn load_sram(&mut self, sram_data: Vec<u8>) {
        self.prg_ram = sram_data;
    }
}
