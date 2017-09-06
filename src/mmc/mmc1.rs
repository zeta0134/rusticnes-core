// A very simple Mapper with no esoteric features or bank switching.
// Reference capabilities: https://wiki.nesdev.com/w/index.php/NROM

use cartridge::NesHeader;
use mmc::mapper::*;

pub struct Mmc1 {
    pub prg_rom: Vec<u8>,
    pub prg_ram: Vec<u8>,
    pub chr_rom: Vec<u8>,

    pub shift_counter: u8,
    pub shift_data: u8,

    // Note: usize is technically overkill, but having the type aligned is
    // just nicer down below.
    pub chr_bank_0: usize,
    pub chr_bank_1: usize,
    pub chr_ram: bool,

    pub prg_bank: usize,
    pub prg_ram_enabled: bool,

    pub control: u8,

    pub mirroring: Mirroring,
}

impl Mmc1 {
    pub fn new(header: NesHeader, chr: &[u8], prg: &[u8]) -> Mmc1 {
        let chr_rom = match header.has_chr_ram {
            true => vec![0u8; 8 * 1024],
            false => chr.to_vec()
        };
        return Mmc1 {
            prg_rom: prg.to_vec(),
            prg_ram: vec![0u8; 0x2000],
            chr_rom: chr_rom,
            shift_counter: 0,
            shift_data: 0,
            chr_bank_0: 0,
            chr_bank_1: 0,
            prg_bank: 0x0F, // Powerup state has all bits set.  This force-loads the last page, no matter the starting mode.
            prg_ram_enabled: true,
            control: 0x0C,
            chr_ram: header.has_chr_ram,
            mirroring: Mirroring::Horizontal, // Completely arbitrary, should be set by game code later
        }
    }
}

impl Mapper for Mmc1 {
    fn print_debug_status(&self) {
        let prg_mode = (self.control >> 2) & 0x3;
        let chr_mode = (self.control & 0x10) >> 4;
        println!("======= MMC1 =======");
        println!("PRG Mode: {} | CHR: Mode: {} | S.Count: {} | S.Data: {:02X}",
            prg_mode, chr_mode, self.shift_counter, self.shift_data);
        let last_bank = (self.prg_rom.len() / (16 * 1024)) as u16 - 1;
        println!("PRG: {} | CHR0: {} | CHR1: {} | PRG_LAST: {}",
            self.prg_bank, self.chr_bank_0, self.chr_bank_1, last_bank);
        println!("====================");
    }

    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }

    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000 ... 0x0FFF => {
                let chr_rom_len = self.chr_rom.len();
                if self.control & 0x10 == 0 {
                    // 8kb CHR mode, bit 0 is ignored
                    let bank = self.chr_bank_0 & 0xFFFE;
                    return self.chr_rom[((bank * 0x1000) + address as usize) % chr_rom_len];
                } else {
                    // 4kb CHR mode
                    return self.chr_rom[((self.chr_bank_0 * 0x1000) + address as usize) % chr_rom_len];
                }
            },
            0x1000 ... 0x1FFF => {
                let chr_rom_len = self.chr_rom.len();
                if self.control & 0x10 == 0 {
                    // 8kb CHR mode, use chr_bank_0 with bit 1 set
                    let bank = self.chr_bank_0 | 0x0001;
                    return self.chr_rom[((bank * 0x1000) +  (address as usize - 0x1000)) % chr_rom_len];
                } else {
                    // 4kb CHR mode
                    return self.chr_rom[((self.chr_bank_1 * 0x1000) +  (address as usize - 0x1000)) % chr_rom_len];
                }
            },
            0x6000 ... 0x7FFF => {
                let prg_ram_len = self.prg_ram.len();
                if prg_ram_len > 0 {
                    return self.prg_ram[((address - 0x6000) % (prg_ram_len as u16)) as usize];
                } else {
                    return 0;
                }
            },
            0x8000 ... 0xBFFF => {
                let prg_rom_len = self.prg_rom.len();
                if prg_rom_len > 0 {
                    let prg_mode = (self.control >> 2) & 0x3;
                    match prg_mode {
                        0 | 1 => {
                            // 32kb PRG mode, use prg_bank ignoring bit 0
                            let bank = self.prg_bank & 0xFFFE;
                            return self.prg_rom[((bank * 0x4000) + (address as usize - 0x8000)) % prg_rom_len];
                        },
                        2 => {
                            // Fixed first bank, read that out here
                            return self.prg_rom[(address - 0x8000) as usize];
                        },
                        3 => {
                            // Fixed last bank, read out the bank-switched first bank
                            return self.prg_rom[((self.prg_bank * 0x4000) + (address as usize - 0x8000)) % prg_rom_len];
                        },
                        _ => return 0, // Never called
                    }
                } else {
                    return 0;
                }
            },
            0xC000 ... 0xFFFF => {
                let prg_rom_len = self.prg_rom.len();
                if prg_rom_len > 0 {
                    let prg_mode = (self.control >> 2) & 0x3;
                    match prg_mode {
                        0 | 1 => {
                            // 32kb PRG mode, use prg_bank and force-set bit 1
                            let bank = self.prg_bank | 0x0001;
                            return self.prg_rom[((bank * 0x4000) + (address as usize - 0xC000)) % prg_rom_len];
                        },
                        2 => {
                            // Fixed first bank, read out the bank-switched second bank
                            return self.prg_rom[((self.prg_bank * 0x4000) + (address as usize - 0xC000)) % prg_rom_len];
                        },
                        3 => {
                            // Fixed last bank, read out the bank-switched *last* bank
                            let last_bank = (self.prg_rom.len() / (16 * 1024)) - 1;
                            return self.prg_rom[((last_bank * 0x4000) + (address as usize - 0xC000))];
                        },
                        _ => return 0, // Never called
                    }
                } else {
                    return 0;
                }
            },
            _ => return 0
        }
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            0x0000 ... 0x0FFF => {
                if self.chr_ram {
                    let chr_rom_len = self.chr_rom.len();
                    if self.control & 0x10 == 0 {
                        // 8kb CHR mode, bit 0 is ignored
                        let bank = self.chr_bank_0 & 0xFFFE;
                        self.chr_rom[((bank * 0x1000) + address as usize) % chr_rom_len] = data;
                    } else {
                        // 4kb CHR mode
                        self.chr_rom[((self.chr_bank_0 * 0x1000) + address as usize) % chr_rom_len] = data;
                    }
                }
            },
            0x1000 ... 0x1FFF => {
                if self.chr_ram {
                    let chr_rom_len = self.chr_rom.len();
                    if self.control & 0x10 == 0 {
                        // 8kb CHR mode, use chr_bank_0 with bit 1 set
                        let bank = self.chr_bank_0 | 0x0001;
                        self.chr_rom[((bank * 0x1000) +  (address as usize - 0x1000)) % chr_rom_len] = data;
                    } else {
                        // 4kb CHR mode
                        self.chr_rom[((self.chr_bank_1 * 0x1000) + (address as usize - 0x1000)) % chr_rom_len] = data;
                    }
                }
            },
            0x6000 ... 0x7FFF => {
                if self.prg_ram_enabled {
                    let prg_ram_len = self.prg_ram.len();
                    if prg_ram_len > 0 {
                        self.prg_ram[((address as usize - 0x6000) % (prg_ram_len))] = data;
                    }
                }
            },
            0x8000 ... 0xFFFF => {
                if data & 0x80 != 0 {
                    // Shift / Control Reset!
                    self.shift_counter = 0;
                    self.control = self.control | 0x0C;
                } else {
                    self.shift_data = (self.shift_data >> 1) | ((data & 0x1) << 4);
                    self.shift_counter += 1;
                    if self.shift_counter == 5 {
                        let register = (address & 0xE000) >> 8;
                        match register {
                            0x80 ... 0x9F => {
                                self.control = self.shift_data;
                                let nametable_mode = (self.control & 0x3);
                                match nametable_mode {
                                    0 => self.mirroring = Mirroring::OneScreenLower,
                                    1 => self.mirroring = Mirroring::OneScreenUpper,
                                    2 => self.mirroring = Mirroring::Vertical,
                                    3 => self.mirroring = Mirroring::Horizontal,
                                    _ => println!("Bad mirroring mode!! {}", nametable_mode) // should never be called
                                }
                            },
                            0xA0 ... 0xBF => self.chr_bank_0 = self.shift_data as usize,
                            0xC0 ... 0xDF => self.chr_bank_1 = self.shift_data as usize,
                            0xE0 ... 0xFF => {
                                self.prg_ram_enabled = self.shift_data & 0x10 != 0;
                                self.prg_bank = (self.shift_data & 0x0F) as usize;
                            },
                            _ => ()
                        }
                        self.shift_counter = 0;
                        self.shift_data = 0;
                    }
                }
            }
            _ => {}
        }
    }
}
