// A very simple Mapper with no esoteric features or bank switching.
// Reference capabilities: https://wiki.nesdev.com/w/index.php/NROM

use cartridge::NesHeader;
use mmc::mapper::*;

pub struct Nrom {
    pub prg_rom: Vec<u8>,
    pub prg_ram: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mirroring: Mirroring,
    pub has_chr_ram: bool,
}

impl Nrom {
    pub fn new(header: NesHeader, chr: &[u8], prg: &[u8]) -> Nrom {
        let chr_rom = match header.has_chr_ram {
            true => vec![0u8; 8 * 1024],
            false => chr.to_vec()
        };

        return Nrom {
            prg_rom: prg.to_vec(),
            prg_ram: vec![0u8; 8 * 1024],
            chr_rom: chr_rom,
            mirroring: header.mirroring,
            has_chr_ram: header.has_chr_ram,
        }
    }
}

impl Mapper for Nrom {
    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }
    
    fn read_cpu(&mut self, address: u16) -> Option<u8> {
        match address {
            0x6000 ... 0x7FFF => {
                let prg_ram_len = self.prg_ram.len();
                if prg_ram_len > 0 {
                    return Some(self.prg_ram[((address - 0x6000) % (prg_ram_len as u16)) as usize]);
                } else {
                    return None;
                }
            },
            0x8000 ... 0xFFFF => {
                let prg_rom_len = self.prg_rom.len();
                return Some(self.prg_rom[(address % (prg_rom_len as u16)) as usize]);
            },
            _ => return None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
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

    fn read_ppu(&mut self, address: u16) -> Option<u8> {
        match address {
            0x0000 ... 0x1FFF => return Some(self.chr_rom[address as usize]),
            _ => return None
        }
    }

    fn write_ppu(&mut self, address: u16, data: u8) {
        match address {
            0x0000 ... 0x1FFF => {
                if self.has_chr_ram {
                    self.chr_rom[address as usize] = data;
                }
            },
            _ => {}
        }
    }
}
