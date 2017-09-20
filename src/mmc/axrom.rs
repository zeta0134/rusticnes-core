// AxROM, bank switchable PRG ROM, 8kb CHR RAM, basic single-screen mirroring.
// Reference capabilities: https://wiki.nesdev.com/w/index.php/AxROM

use cartridge::NesHeader;
use mmc::mapper::*;

pub struct AxRom {
    pub prg_rom: Vec<u8>,
    pub chr_ram: Vec<u8>,
    pub mirroring: Mirroring,
    pub prg_bank: usize,
}

impl AxRom {
    pub fn new(_: NesHeader, _: &[u8], prg: &[u8]) -> AxRom {
        return AxRom {
            prg_rom: prg.to_vec(),
            chr_ram: vec![0u8; 0x2000],
            mirroring: Mirroring::OneScreenUpper,
            prg_bank: 0x07,
        }
    }
}

impl Mapper for AxRom {
    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }

    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000 ... 0x1FFF => return self.chr_ram[address as usize],
            0x8000 ... 0xFFFF => {
                let prg_rom_len = self.prg_rom.len();
                return self.prg_rom[((self.prg_bank * 0x8000) + (address as usize - 0x8000)) % prg_rom_len];
            },
            _ => return 0
        }
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            0x0000 ... 0x1FFF => self.chr_ram[address as usize] = data,
            0x8000 ... 0xFFFF => {
                self.prg_bank = (data & 0x07) as usize;
                if data & 0x10 == 0 {
                    self.mirroring = Mirroring::OneScreenLower;
                } else {
                    self.mirroring = Mirroring::OneScreenUpper;
                }
            }
            _ => {}
        }
    }
}
