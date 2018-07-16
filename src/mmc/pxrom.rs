// MMC2, a somewhat advanced bank switcher with extended CHR memory
// https://wiki.nesdev.com/w/index.php/MMC2

use cartridge::NesHeader;
use mmc::mapper::*;

pub struct PxRom {
    pub prg_rom: Vec<u8>,
    pub prg_ram: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mirroring: Mirroring,
    pub chr_0_latch: u8,
    pub chr_0_fd_bank: usize,
    pub chr_0_fe_bank: usize,
    pub chr_1_latch: u8,
    pub chr_1_fd_bank: usize,
    pub chr_1_fe_bank: usize,
    pub prg_bank: usize,
}

impl PxRom {
    pub fn new(_header: NesHeader, chr: &[u8], prg: &[u8]) -> PxRom {
        return PxRom {
            prg_rom: prg.to_vec(),
            prg_ram: vec![0u8; 8 * 1024],
            chr_rom: chr.to_vec(),
            mirroring: Mirroring::Vertical,
            chr_0_latch: 0,
            chr_0_fd_bank: 0,
            chr_0_fe_bank: 0,
            chr_1_latch: 0,
            chr_1_fd_bank: 0,
            chr_1_fe_bank: 0,
            prg_bank: 0,
        }
    }
}

impl Mapper for PxRom {
    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }
    
    fn read_byte(&mut self, address: u16) -> Option<u8> {
        match address {
            0x0000 ... 0x0FFF => {
                let chr_rom_len = self.chr_rom.len();
                let chr_bank = match self.chr_0_latch {
                    0 => self.chr_0_fd_bank,
                    1 => self.chr_0_fe_bank,
                    _ => 0
                };
                let chr_byte = Some(self.chr_rom[((chr_bank * 0x1000) + (address as usize)) % chr_rom_len]);
                match address {
                    0x0FD8 => {self.chr_0_latch = 0;},
                    0x0FE8 => {self.chr_0_latch = 1;},
                    _ => {}
                }
                return chr_byte;
            },
            0x1000 ... 0x1FFF => {
                let chr_rom_len = self.chr_rom.len();
                let chr_bank = match self.chr_1_latch {
                    0 => self.chr_1_fd_bank,
                    1 => self.chr_1_fe_bank,
                    _ => 0
                };
                let chr_byte = Some(self.chr_rom[((chr_bank * 0x1000) + ((address - 0x1000) as usize)) % chr_rom_len]);
                match address {
                    0x1FD8 ... 0x1FDF => {self.chr_1_latch = 0;},
                    0x1FE8 ... 0x1FEF => {self.chr_1_latch = 1;},
                    _ => {}
                }
                return chr_byte;
            },
            0x6000 ... 0x7FFF => {
                let prg_ram_len = self.prg_ram.len();
                if prg_ram_len > 0 {
                    return Some(self.prg_ram[((address - 0x6000) % (prg_ram_len as u16)) as usize]);
                } else {
                    return None;
                }
            },
            0x8000 ... 0x9FFF => {
                let prg_rom_len = self.prg_rom.len();
                return Some(self.prg_rom[((self.prg_bank * 0x2000) + (address as usize - 0x8000)) % prg_rom_len]);
            },
            0xA000 ... 0xFFFF => {
                let prg_rom_len = self.prg_rom.len();
                return Some(self.prg_rom[((prg_rom_len - 0x6000) + (address as usize -  0xA000)) % prg_rom_len])
            },
            _ => return None
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
            0xA000 ... 0xAFFF => { self.prg_bank = (data & 0b0000_1111) as usize; },
            0xB000 ... 0xBFFF => { self.chr_0_fd_bank = (data & 0b0001_1111) as usize; },
            0xC000 ... 0xCFFF => { self.chr_0_fe_bank = (data & 0b0001_1111) as usize; },
            0xD000 ... 0xDFFF => { self.chr_1_fd_bank = (data & 0b0001_1111) as usize; },
            0xE000 ... 0xEFFF => { self.chr_1_fe_bank = (data & 0b0001_1111) as usize; },
            0xF000 ... 0xFFFF => { 
                if data & 0b1 == 0 {
                    self.mirroring = Mirroring::Vertical;
                } else {
                    self.mirroring = Mirroring::Horizontal;
                }
            },
            _ => {}
        }
    }
}
