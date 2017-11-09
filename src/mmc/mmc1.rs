// Common mapper with bank switched PRG_ROM, CHR_ROM/RAM, and optional PRG RAM.
// Reference capabilities: https://wiki.nesdev.com/w/index.php/MMC1

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
    pub last_write: bool,
}

impl Mmc1 {
    pub fn new(header: NesHeader, chr: &[u8], prg: &[u8]) -> Mmc1 {
        // If the header signals that we're using CHR_RAM, then the cartridge loader will
        // have provided an empty "chr_rom" vector, so create a new empty 8k vector instead.
        let chr_rom = match header.has_chr_ram {
            true => vec![0u8; 8 * 1024],
            false => chr.to_vec()
        };

        return Mmc1 {

            prg_rom: prg.to_vec(),
            prg_ram: vec![0u8; 0x2000],
            chr_rom: chr_rom,
            chr_ram: header.has_chr_ram,
            // Note: On real MMC1-based hardware, many of these values are random on startup, so
            // the defaults presented below are arbitrary.
            shift_counter: 0,
            shift_data: 0,
            chr_bank_0: 0,
            chr_bank_1: 0,
            // Powerup and reset always have all bits set. This force-loads the last prg page, so
            // programs can reliably place their reset routines here.
            prg_bank: 0x00,
            prg_ram_enabled: true,
            control: 0x0C,
            mirroring: Mirroring::Vertical,
            last_write: false,
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

    fn read_byte(&mut self, address: u16) -> u8 {
        self.last_write = false;
        match address {
            // CHR Bank 0
            0x0000 ... 0x0FFF => {
                let chr_rom_len = self.chr_rom.len();
                if self.control & 0x10 == 0 {
                    // 8kb CHR mode, bit 0 is treated as cleared
                    let bank = self.chr_bank_0 & 0xFFFE;
                    return self.chr_rom[((bank * 0x1000) + address as usize) % chr_rom_len];
                } else {
                    // 4kb CHR mode
                    return self.chr_rom[((self.chr_bank_0 * 0x1000) + address as usize) % chr_rom_len];
                }
            },
            // CHR Bank 1
            0x1000 ... 0x1FFF => {
                let chr_rom_len = self.chr_rom.len();
                if self.control & 0x10 == 0 {
                    // 8kb CHR mode, bit 0 is treated as set
                    let bank = self.chr_bank_0 | 0x0001;
                    return self.chr_rom[((bank * 0x1000) +  (address as usize - 0x1000)) % chr_rom_len];
                } else {
                    // 4kb CHR mode
                    return self.chr_rom[((self.chr_bank_1 * 0x1000) +  (address as usize - 0x1000)) % chr_rom_len];
                }
            },
            // PRG RAM
            0x6000 ... 0x7FFF => {
                let prg_ram_len = self.prg_ram.len();
                if prg_ram_len > 0 {
                    return self.prg_ram[(address - 0x6000) as usize];
                } else {
                    return 0;
                }
            },
            // PRG ROM - First 16k Page
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
            // PRG ROM - Last 16k Page
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
            // CHR Bank 0
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
            // CHR Bank 1
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
            // PRG RAM
            0x6000 ... 0x7FFF => {
                if self.prg_ram_enabled {
                    let prg_ram_len = self.prg_ram.len();
                    if prg_ram_len > 0 {
                        self.prg_ram[address as usize - 0x6000] = data;
                    }
                }
            },
            // Control Registers
            0x8000 ... 0xFFFF => {
                if self.last_write {
                    // Ignore this write! MMC1 ignores successive writes, and will clear this flag
                    // on the next read cycle.
                    return;
                }
                self.last_write = true;
                
                if data & 0x80 != 0 {
                    // Shift / Control Reset!
                    self.shift_counter = 0;
                    // Upon reset, this sets the PRG ROM mode to 3, which fixes the last bank
                    // to the upper PRG Page. This is the startup state of MMC1 variants.
                    // https://wiki.nesdev.com/w/index.php/MMC1#Load_register_.28.248000-.24FFFF.29
                    self.control = self.control | 0b0_1100;
                } else {
                    self.shift_data = (self.shift_data >> 1) | ((data & 0b1) << 4);
                    self.shift_counter += 1;
                    if self.shift_counter == 5 {
                        // Only the top 3 bits (13-15) matter for register selection, everything
                        // else is mirrored due to incomplete decoding of the address.
                        // https://wiki.nesdev.com/w/index.php/MMC1#Registers
                        let register = address & 0b1110_0000_0000_0000;
                        match register {
                            0x8000 ... 0x9F00 => {
                                self.control = self.shift_data;
                                let nametable_mode = self.control & 0b0_0011;
                                match nametable_mode {
                                    0 => self.mirroring = Mirroring::OneScreenLower,
                                    1 => self.mirroring = Mirroring::OneScreenUpper,
                                    2 => self.mirroring = Mirroring::Vertical,
                                    3 => self.mirroring = Mirroring::Horizontal,
                                    _ => println!("Bad mirroring mode!! {}", nametable_mode),
                                }
                            },
                            0xA000 ... 0xBF00 => self.chr_bank_0 = self.shift_data as usize,
                            0xC000 ... 0xDF00 => self.chr_bank_1 = self.shift_data as usize,
                            0xE000 ... 0xFF00 => {
                                // The 5th bit disables RAM, so invert it here to decide when
                                // RAM should be enabled.
                                // TODO: This is ignored on certain MMC variants!
                                self.prg_ram_enabled = self.shift_data & 0b1_0000 == 0;
                                self.prg_bank = (self.shift_data & 0b0_1111) as usize;
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
