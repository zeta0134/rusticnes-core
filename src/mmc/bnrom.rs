// BNROM, bank switchable PRG ROM, 8kb CHR RAM, solder-pad fixed horizontal or vertical mirroring.
// Essentially an AxROM variant, though I'm choosing to keep all numbered mapper implementations 
// dependency free for my own sanity.
// Reference capabilities: https://wiki.nesdev.com/w/index.php/BNROM

use cartridge::NesHeader;
use mmc::mapper::*;
use mmc::mirroring;

pub struct BnRom {
    pub prg_rom: Vec<u8>,
    pub chr_ram: Vec<u8>,
    pub mirroring: Mirroring,
    pub prg_bank: usize,
    pub vram: Vec<u8>,
}

impl BnRom {
    pub fn new(header: NesHeader, _: &[u8], prg: &[u8]) -> BnRom {
        return BnRom {
            prg_rom: prg.to_vec(),
            chr_ram: vec![0u8; 0x2000],
            mirroring: header.mirroring,
            prg_bank: 0x07,
            vram: vec![0u8; 0x1000],
        }
    }
}

impl Mapper for BnRom {
    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }

    fn print_debug_status(&self) {
        println!("======= BNROM =======");
        println!("PRG Bank: {}, Mirroring Mode: {}", self.prg_bank, mirroring_mode_name(self.mirroring));
        println!("====================");
    }

    fn read_cpu(&mut self, address: u16) -> Option<u8> {
        match address {
            0x8000 ... 0xFFFF => {
                let prg_rom_len = self.prg_rom.len();
                return Some(self.prg_rom[((self.prg_bank * 0x8000) + (address as usize - 0x8000)) % prg_rom_len]);
            },
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
