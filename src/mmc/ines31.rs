// iNES Mapper 031 represents a mapper created to facilitate cartridge compilations 
// of NSF music. It implements a common subset of the features used by NSFs. 
// Reference capabilities: https://wiki.nesdev.com/w/index.php/INES_Mapper_031

use cartridge::NesHeader;
use mmc::mapper::*;
use mmc::mirroring;

pub struct INes31 {
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mirroring: Mirroring,
    pub has_chr_ram: bool,
    pub vram: Vec<u8>,
    pub prg_banks: Vec<usize>,
}

impl INes31 {
    pub fn new(header: NesHeader, chr: &[u8], prg: &[u8]) -> INes31 {
        let chr_rom = match header.has_chr_ram {
            true => vec![0u8; 8 * 1024],
            false => chr.to_vec()
        };

        return INes31 {
            prg_rom: prg.to_vec(),
            chr_rom: chr_rom,
            mirroring: header.mirroring,
            has_chr_ram: header.has_chr_ram,
            vram: vec![0u8; 0x1000],
            prg_banks: vec![255usize; 8],
        }
    }
}

impl Mapper for INes31 {
    fn print_debug_status(&self) {
        println!("======= iNes 31 =======");
        println!("Mirroring Mode: {}", mirroring_mode_name(self.mirroring));
        println!("====================");
    }

    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }
    
    fn debug_read_cpu(&self, address: u16) -> Option<u8> {
        let prg_rom_len = self.prg_rom.len();
        match address {
            0x8000 ..= 0x8FFF => return Some(self.prg_rom[((self.prg_banks[0] * 0x1000)    + (address as usize -  0x8000)) % prg_rom_len]),
            0x9000 ..= 0x9FFF => return Some(self.prg_rom[((self.prg_banks[1] * 0x1000)    + (address as usize -  0x9000)) % prg_rom_len]),
            0xA000 ..= 0xAFFF => return Some(self.prg_rom[((self.prg_banks[2] * 0x1000)    + (address as usize -  0xA000)) % prg_rom_len]),
            0xB000 ..= 0xBFFF => return Some(self.prg_rom[((self.prg_banks[3] * 0x1000)    + (address as usize -  0xB000)) % prg_rom_len]),
            0xC000 ..= 0xCFFF => return Some(self.prg_rom[((self.prg_banks[4] * 0x1000)    + (address as usize -  0xC000)) % prg_rom_len]),
            0xD000 ..= 0xDFFF => return Some(self.prg_rom[((self.prg_banks[5] * 0x1000)    + (address as usize -  0xD000)) % prg_rom_len]),
            0xE000 ..= 0xEFFF => return Some(self.prg_rom[((self.prg_banks[6] * 0x1000)    + (address as usize -  0xE000)) % prg_rom_len]),
            0xF000 ..= 0xFFFF => return Some(self.prg_rom[((self.prg_banks[7] * 0x1000)    + (address as usize -  0xF000)) % prg_rom_len]),
            _ => return None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        match address {
            0x5FF8 => {self.prg_banks[0] = data as usize},
            0x5FF9 => {self.prg_banks[1] = data as usize},
            0x5FFA => {self.prg_banks[2] = data as usize},
            0x5FFB => {self.prg_banks[3] = data as usize},
            0x5FFC => {self.prg_banks[4] = data as usize},
            0x5FFD => {self.prg_banks[5] = data as usize},
            0x5FFE => {self.prg_banks[6] = data as usize},
            0x5FFF => {self.prg_banks[7] = data as usize},
            _ => {}
        }
    }

    fn debug_read_ppu(&self, address: u16) -> Option<u8> {
        match address {
            0x0000 ..= 0x1FFF => return Some(self.chr_rom[address as usize]),
            0x2000 ..= 0x3FFF => return match self.mirroring {
                Mirroring::Horizontal => Some(self.vram[mirroring::horizontal_mirroring(address) as usize]),
                Mirroring::Vertical   => Some(self.vram[mirroring::vertical_mirroring(address) as usize]),
                _ => None
            },
            _ => return None
        }
    }

    fn write_ppu(&mut self, address: u16, data: u8) {
        match address {
            0x0000 ..= 0x1FFF => {
                if self.has_chr_ram {
                    self.chr_rom[address as usize] = data;
                }
            },
            0x2000 ..= 0x3FFF => match self.mirroring {
                Mirroring::Horizontal => self.vram[mirroring::horizontal_mirroring(address) as usize] = data,
                Mirroring::Vertical   => self.vram[mirroring::vertical_mirroring(address) as usize] = data,
                _ => {}
            },
            _ => {}
        }
    }
}
