// A very simple Mapper with no esoteric features or bank switching.
// Reference capabilities: https://wiki.nesdev.com/w/index.php/NROM

use cartridge::NesHeader;
use mmc::mapper::Mapper;

pub struct Mmc1 {
    pub prg_rom: Vec<u8>,
    pub prg_ram: Vec<u8>,
    pub chr_rom: Vec<u8>,

    pub shift_counter: u8,
    pub shift_data: u8,

    // Note: u16 is technically overkill, but having the type aligned
    // with u16 addresses makes typecasting nicer.
    pub chr_bank_0: u16,
    pub chr_bank_1: u16,

    pub prg_bank: u16,
    pub prg_ram_enabled: bool,

    pub control: u8,
}

impl Mmc1 {
    pub fn new(header: NesHeader, chr: &[u8], prg: &[u8]) -> Mmc1 {
        println!("Creating MMC1!!");
        println!("PRG: {0}, CHR: {1}", prg.to_vec().len(), chr.to_vec().len());
        return Mmc1 {
            prg_rom: prg.to_vec(),
            prg_ram: vec![0u8; 0x2000],
            chr_rom: chr.to_vec(),
            shift_counter: 0,
            shift_data: 0,
            chr_bank_0: 0,
            chr_bank_1: 0,
            prg_bank: 0,
            prg_ram_enabled: false,
            control: 0,
        }
    }
}

impl Mapper for Mmc1 {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000 ... 0x0FFF => {
                let chr_rom_len = self.chr_rom.len();
                if self.control & 0x10 == 0 {
                    // 8kb CHR mode, bit 0 is ignored
                    let bank = self.chr_bank_0 & 0xFFFE;
                    return self.chr_rom[((bank * 0x1000) + address) as usize % chr_rom_len];
                } else {
                    // 4kb CHR mode
                    return self.chr_rom[((self.chr_bank_0 * 0x1000) + address) as usize % chr_rom_len];
                }
            },
            0x1000 ... 0x1FFF => {
                let chr_rom_len = self.chr_rom.len();
                if self.control & 0x10 == 0 {
                    // 8kb CHR mode, use chr_bank_0 with bit 1 set
                    let bank = self.chr_bank_0 | 0x0001;
                    return self.chr_rom[((bank * 0x1000) +  (address - 0x1000)) as usize % chr_rom_len];
                } else {
                    // 4kb CHR mode
                    return self.chr_rom[((self.chr_bank_1 * 0x1000) +  (address - 0x1000)) as usize % chr_rom_len];
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
                            return self.prg_rom[((bank * 0x4000) + (address - 0x8000)) as usize % prg_rom_len];
                        },
                        2 => {
                            // Fixed first bank, read that out here
                            return self.prg_rom[(address - 0x8000) as usize];
                        },
                        3 => {
                            // Fixed last bank, read out the bank-switched first bank
                            return self.prg_rom[((self.prg_bank * 0x4000) + (address - 0x8000)) as usize % prg_rom_len];
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
                            return self.prg_rom[((bank * 0x4000) + (address - 0xC000)) as usize % prg_rom_len];
                        },
                        2 => {
                            // Fixed first bank, read out the bank-switched second bank
                            return self.prg_rom[((self.prg_bank * 0x4000) + (address - 0xC000)) as usize % prg_rom_len];
                        },
                        3 => {
                            // Fixed last bank, read out the bank-switched *last* bank
                            let last_bank = (self.prg_rom.len() / (16 * 1024)) as u16 - 1;
                            return self.prg_rom[((last_bank * 0x4000) + (address - 0xC000)) as usize];
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
            0x6000 ... 0x7FFF => {
                let prg_ram_len = self.prg_ram.len();
                if prg_ram_len > 0 {
                    self.prg_ram[((address - 0x6000) % (prg_ram_len as u16)) as usize] = data;
                }
            },
            _ => {}
        }
    }
}
