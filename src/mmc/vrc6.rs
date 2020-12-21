// Vrc6, 
// Reference capabilities: https://wiki.nesdev.com/w/index.php/VRC6

use cartridge::NesHeader;
use mmc::mapper::*;

pub struct Vrc6 {
    pub prg_rom: Vec<u8>,
    pub prg_ram: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mirroring: Mirroring,
    pub vram: Vec<u8>,
    pub prg_ram_enable: bool
}

impl Vrc6 {
    pub fn new(header: NesHeader, chr: &[u8], prg: &[u8]) -> Vrc6 {
        return Vrc6 {
            prg_rom: prg.to_vec(),
            prg_ram: vec![0u8; 8 * 1024],
            chr_rom: chr.to_vec(),
            mirroring: header.mirroring,
            vram: vec![0u8; 0x1000],
            prg_ram_enable: false,
        }
    }
}

impl Mapper for Vrc6 {
    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }

    fn read_cpu(&mut self, address: u16) -> Option<u8> {
        match address {
            0x6000 ..= 0x7FFF => {
                return Some(self.prg_ram[(address - 0x6000) as usize]);
            },
            _ => return None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        match address {
            0x6000 ..= 0x7FFF => {
                if self.prg_ram_enable {
                    self.prg_ram[(address - 0x6000) as usize] = data;
                }
            },

            _ => {}
        }
    }

    fn read_ppu(&mut self, address: u16) -> Option<u8> {
        match address {
            _ => return None
        }
    }

    fn write_ppu(&mut self, address: u16, data: u8) {
        match address {
            _ => {}
        }
    }
}