// Advanced mapper with bank-switched PRG ROM and CHR ROM, and a scanline counter feeding into IRQ
// Reference capabilities: https://wiki.nesdev.com/w/index.php/MMC1

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
    pub fn new(_: NesHeader, chr: &[u8], prg: &[u8]) -> Mmc3 {
        return Mmc3 {
            prg_rom: prg.to_vec(),
            prg_ram: vec![0u8; 0x2000],
            chr_rom: chr.to_vec(),
            // Note: On real MMC3-based hardware, many of these values are random on startup, so
            // the defaults presented below are arbitrary.
            chr2_bank_0: 0,
            chr2_bank_1: 0,
            chr1_bank_2: 0,
            chr1_bank_3: 0,
            chr1_bank_4: 0,
            chr1_bank_5: 0,

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

    fn _read_byte(&mut self, address: u16, side_effects: bool) -> u8 {
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
                if self.switch_chr_banks {
                    match address {
                        0x0000 ... 0x03FF => return self.chr_rom[(self.chr1_bank_2 * 0x400) + (address as usize -  0x000)],
                        0x0400 ... 0x07FF => return self.chr_rom[(self.chr1_bank_3 * 0x400) + (address as usize -  0x400)],
                        0x0800 ... 0x0BFF => return self.chr_rom[(self.chr1_bank_4 * 0x400) + (address as usize -  0x800)],
                        0x0C00 ... 0x0FFF => return self.chr_rom[(self.chr1_bank_5 * 0x400) + (address as usize -  0xC00)],
                        0x1000 ... 0x17FF => return self.chr_rom[(self.chr2_bank_0 * 0x400) + (address as usize - 0x1000)],
                        0x1800 ... 0x1FFF => return self.chr_rom[(self.chr2_bank_1 * 0x400) + (address as usize - 0x1800)],
                        _ => return 0,
                    }
                } else {
                    match address {
                        0x0000 ... 0x07FF => return self.chr_rom[(self.chr2_bank_0 * 0x400) + (address as usize -  0x000)],
                        0x0800 ... 0x0FFF => return self.chr_rom[(self.chr2_bank_1 * 0x400) + (address as usize -  0x800)],
                        0x1000 ... 0x13FF => return self.chr_rom[(self.chr1_bank_2 * 0x400) + (address as usize - 0x1000)],
                        0x1400 ... 0x17FF => return self.chr_rom[(self.chr1_bank_3 * 0x400) + (address as usize - 0x1400)],
                        0x1800 ... 0x1BFF => return self.chr_rom[(self.chr1_bank_4 * 0x400) + (address as usize - 0x1800)],
                        0x1C00 ... 0x1FFF => return self.chr_rom[(self.chr1_bank_5 * 0x400) + (address as usize - 0x1C00)],
                        _ => return 0,
                    }
                }
            },
            // PRG RAM
            0x6000 ... 0x7FFF => {
                return self.prg_ram[(address - 0x6000) as usize];
            },
            // PRG ROM
            0x8000 ... 0xFFFF => {
                if self.switch_prg_banks {
                    match address {
                        0x8000 ... 0x9FFF => return self.prg_rom[(self.prg_rom.len() - 0x4000) + (address as usize -  0x8000)],
                        0xA000 ... 0xBFFF => return self.prg_rom[(self.prg_bank_7 * 0x2000)    + (address as usize -  0xA000)],
                        0xC000 ... 0xDFFF => return self.prg_rom[(self.prg_bank_6 * 0x2000)    + (address as usize -  0xC000)],
                        0xE000 ... 0xFFFF => return self.prg_rom[(self.prg_rom.len() - 0x2000) + (address as usize -  0xE000)],
                        _ => return 0,
                    }
                } else {
                    match address {
                        0x8000 ... 0x9FFF => return self.prg_rom[(self.prg_bank_6 * 0x2000)    + (address as usize -  0x8000)],
                        0xA000 ... 0xBFFF => return self.prg_rom[(self.prg_bank_7 * 0x2000)    + (address as usize -  0xA000)],
                        0xC000 ... 0xDFFF => return self.prg_rom[(self.prg_rom.len() - 0x4000) + (address as usize -  0xC000)],
                        0xE000 ... 0xFFFF => return self.prg_rom[(self.prg_rom.len() - 0x2000) + (address as usize -  0xE000)],
                        _ => return 0,
                    }
                }
            },
            _ => return 0
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

    fn read_byte(&mut self, address: u16) -> u8 {
        return self._read_byte(address, true);
    }

    fn debug_read_byte(&mut self, address: u16) -> u8 {
        return self._read_byte(address, false);
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        match address {
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
}
