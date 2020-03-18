// UxROM, simple bank switchable PRG ROM with the last page fixed
// Reference capabilities: https://wiki.nesdev.com/w/index.php/UxROM

use cartridge::NesHeader;
use mmc::mapper::*;
use mmc::mirroring;

pub struct UxRom {
    pub prg_rom: Vec<u8>,
    pub chr_ram: Vec<u8>,
    pub mirroring: Mirroring,
    pub prg_bank: usize,
    pub vram: Vec<u8>,
}

impl UxRom {
    pub fn new(header: NesHeader, _: &[u8], prg: &[u8]) -> UxRom {
        return UxRom {
            prg_rom: prg.to_vec(),
            chr_ram: vec![0u8; 0x2000],
            mirroring: header.mirroring,
            prg_bank: 0x00,
            vram: vec![0u8; 0x1000],
        }
    }
}

impl Mapper for UxRom {
    fn print_debug_status(&self) {
        println!("======= UxROM =======");
        println!("PRG Bank: {}, ", self.prg_bank);
        println!("Mirroring Mode: {}", mirroring_mode_name(self.mirroring));
        println!("====================");
    }

    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }

    fn read_cpu(&mut self, address: u16) -> Option<u8> {
        match address {
            0x8000 ... 0xBFFF => {
                let prg_rom_len = self.prg_rom.len();
                return Some(self.prg_rom[((self.prg_bank * 0x4000) + (address as usize - 0x8000)) % prg_rom_len]);
            },
            0xC000 ... 0xFFFF => {
                return Some(self.prg_rom[self.prg_rom.len() - 0x4000 + (address as usize - 0xC000)]);
            }
            _ => return None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        match address {
            0x8000 ... 0xFFFF => {
                self.prg_bank = data as usize;
            }
            _ => {}
        }
    }

    fn read_ppu(&mut self, address: u16) -> Option<u8> {
        match address {
            0x0000 ... 0x1FFF => return Some(self.chr_ram[address as usize]),
            0x2000 ... 0x3FFF => return match self.mirroring {
                Mirroring::Horizontal => Some(self.vram[mirroring::horizontal_mirroring(address) as usize]),
                Mirroring::Vertical   => Some(self.vram[mirroring::vertical_mirroring(address) as usize]),
                _ => None
            },
            _ => return None
        }
    }

    fn write_ppu(&mut self, address: u16, data: u8) {
        match address {
            0x0000 ... 0x1FFF => self.chr_ram[address as usize] = data,
            0x2000 ... 0x3FFF => match self.mirroring {
                Mirroring::Horizontal => self.vram[mirroring::horizontal_mirroring(address) as usize] = data,
                Mirroring::Vertical   => self.vram[mirroring::vertical_mirroring(address) as usize] = data,
                _ => {}
            },
            _ => {}
        }
    }
}
