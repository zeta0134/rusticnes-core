// CnROM, 16-32kb PRG ROM, up to 2048k CHR ROM
// Reference capabilities: https://wiki.nesdev.com/w/index.php/INES_Mapper_003

use cartridge::NesHeader;
use mmc::mapper::*;
use mmc::mirroring;

pub struct CnRom {
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mirroring: Mirroring,
    pub chr_bank: usize,
    pub vram: Vec<u8>,
}

impl CnRom {
    pub fn new(header: NesHeader, chr: &[u8], prg: &[u8]) -> CnRom {
        return CnRom {
            prg_rom: prg.to_vec(),
            chr_rom: chr.to_vec(),
            mirroring: header.mirroring,
            chr_bank: 0x00,
            vram: vec![0u8; 0x1000],
        }
    }
}

impl Mapper for CnRom {
    fn print_debug_status(&self) {
        println!("======= CnROM =======");
        println!("CHR Bank: {}, Mirroring Mode: {}", self.chr_bank, mirroring_mode_name(self.mirroring));
        println!("====================");
    }

    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }

    fn read_cpu(&mut self, address: u16) -> Option<u8> {
        match address {
            0x8000 ... 0xFFFF => {
                let prg_rom_len = self.prg_rom.len();
                return Some(self.prg_rom[(address as usize - 0x8000) % prg_rom_len]);
            },
            _ => return None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        match address {
            0x8000 ... 0xFFFF => {
                self.chr_bank = data as usize;
            }
            _ => {}
        }
    }

    fn read_ppu(&mut self, address: u16) -> Option<u8> {
        match address {
            0x0000 ... 0x1FFF => {
                let chr_rom_len = self.chr_rom.len();
                return Some(self.chr_rom[((self.chr_bank * 0x2000) + (address as usize)) % chr_rom_len]);
            },
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
            0x2000 ... 0x3FFF => match self.mirroring {
                Mirroring::Horizontal => self.vram[mirroring::horizontal_mirroring(address) as usize] = data,
                Mirroring::Vertical   => self.vram[mirroring::vertical_mirroring(address) as usize] = data,
                _ => {}
            },
            _ => {}
        }
    }
}
