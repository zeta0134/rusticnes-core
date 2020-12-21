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
    pub prg_ram_enable: bool,
    pub prg_bank_16: usize,
    pub prg_bank_8: usize,
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
            prg_bank_16: 0,
            prg_bank_8: 0,
        }
    }
}

impl Mapper for Vrc6 {
    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }

    fn read_cpu(&mut self, address: u16) -> Option<u8> {
        let prg_rom_len = self.prg_rom.len();
        match address {
            0x6000 ..= 0x7FFF => {
                return Some(self.prg_ram[(address - 0x6000) as usize]);
            },
            0x8000 ..= 0xBFFF => {
                return Some(self.prg_rom[((self.prg_bank_16 * 0x4000) + (address as usize - 0x8000)) % prg_rom_len]);
            },
            0xC000 ..= 0xDFFF => {
                return Some(self.prg_rom[((self.prg_bank_8 * 0x2000) + (address as usize - 0xC000)) % prg_rom_len]);
            },
            0xE000 ..= 0xFFFF => {
                return Some(self.prg_rom[(prg_rom_len - 0x2000) + (address as usize -  0xE000)]);
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
        let masked_address = address & 0b1111_0000_0000_0011;
        match masked_address {
            0x8000 ..= 0x8003 => {
                self.prg_bank_16 = data as usize & 0x0F;
            },
            0xC000 ..= 0xC003 => {
                self.prg_bank_8 = data as usize & 0x1F;
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