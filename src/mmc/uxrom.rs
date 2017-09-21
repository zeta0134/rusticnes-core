// UxROM, simple bank switchable PRG ROM with the last page fixed
// Reference capabilities: https://wiki.nesdev.com/w/index.php/AxROM

use cartridge::NesHeader;
use mmc::mapper::*;

pub struct UxRom {
    pub prg_rom: Vec<u8>,
    pub chr_ram: Vec<u8>,
    pub mirroring: Mirroring,
    pub prg_bank: usize,
}

impl UxRom {
    pub fn new(header: NesHeader, _: &[u8], prg: &[u8]) -> UxRom {
        return UxRom {
            prg_rom: prg.to_vec(),
            chr_ram: vec![0u8; 0x2000],
            mirroring: header.mirroring,
            prg_bank: 0x00,
        }
    }
}

impl Mapper for UxRom {
    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }

    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000 ... 0x1FFF => return self.chr_ram[address as usize],
            0x8000 ... 0xBFFF => {
                let prg_rom_len = self.prg_rom.len();
                return self.prg_rom[((self.prg_bank * 0x4000) + (address as usize - 0x8000)) % prg_rom_len];
            },
            0xC000 ... 0xFFFF => {
                return self.prg_rom[self.prg_rom.len() - 0x4000 + (address as usize - 0xC000)];
            }
            _ => return 0
        }
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            0x0000 ... 0x1FFF => self.chr_ram[address as usize] = data,
            0x8000 ... 0xFFFF => {
                self.prg_bank = data as usize;
            }
            _ => {}
        }
    }
}
