// Sunsoft FME-7, 5A, and 5B (notably lacking expansion audio for now)
// Reference implementation: https://wiki.nesdev.com/w/index.php/Sunsoft_FME-7

use cartridge::NesHeader;
use mmc::mapper::*;
use mmc::mirroring;

pub struct Fme7 {
    pub prg_rom: Vec<u8>,
    pub prg_ram: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub command: u8,
    pub chr_banks: Vec<usize>,
    pub prg_banks: Vec<usize>,
    pub prg_ram_enabled: bool,
    pub prg_ram_selected: bool,
    pub vram: Vec<u8>,
    pub mirroring: Mirroring,
}

impl Fme7 {
    pub fn new(header: NesHeader, chr: &[u8], prg: &[u8]) -> Fme7 {
        return Fme7 {
            prg_rom: prg.to_vec(),
            chr_rom: chr.to_vec(),
            prg_ram: vec![0u8; header.prg_ram_size],
            command: 0,
            chr_banks: vec![0usize; 8],
            prg_banks: vec![0usize; 4],
            prg_ram_enabled: false,
            prg_ram_selected: false,
            vram: vec![0u8; 0x1000],
            mirroring: Mirroring::Vertical,
        }
    }
}

impl Mapper for Fme7 {
    fn mirroring(&self) -> Mirroring {
        return Mirroring::Horizontal;
    }
    
    fn read_cpu(&mut self, address: u16) -> Option<u8> {
        let prg_rom_len = self.prg_rom.len();
        let prg_ram_len = self.prg_ram.len();
        match address {
            0x6000 ..= 0x7FFF => {
                if self.prg_ram_selected {
                    if self.prg_ram_enabled {
                        return Some(self.prg_ram[((self.prg_banks[0] * 0x2000) + (address as usize - 0x6000)) % prg_ram_len]);
                    } else {
                        return None
                    }
                } else {
                    return Some(self.prg_rom[((self.prg_banks[0] * 0x2000) + (address as usize - 0x6000)) % prg_ram_len]);
                }
            },
            0x8000 ..= 0x9FFF => return Some(self.prg_rom[((self.prg_banks[1] * 0x2000) + (address as usize - 0x8000)) % prg_rom_len]),
            0xA000 ..= 0xBFFF => return Some(self.prg_rom[((self.prg_banks[2] * 0x2000) + (address as usize - 0xA000)) % prg_rom_len]),
            0xC000 ..= 0xDFFF => return Some(self.prg_rom[((self.prg_banks[3] * 0x2000) + (address as usize - 0xC000)) % prg_rom_len]),
            0xE000 ..= 0xFFFF => return Some(self.prg_rom[(prg_rom_len - 0x2000) + (address as usize - 0xE000)]),
            _ => return None
        }
    }

    fn read_ppu(&mut self, address: u16) -> Option<u8> {
        match address {
            0x2000 ..= 0x3FFF => return match self.mirroring {
                Mirroring::Horizontal => Some(self.vram[mirroring::horizontal_mirroring(address) as usize]),
                Mirroring::Vertical   => Some(self.vram[mirroring::vertical_mirroring(address) as usize]),
                Mirroring::OneScreenLower => Some(self.vram[mirroring::one_screen_lower(address) as usize]),
                Mirroring::OneScreenUpper => Some(self.vram[mirroring::one_screen_upper(address) as usize]),
                _ => None
            },
            _ => return None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        match address {
            0x6000 ..= 0x7FFF => {
                if self.prg_ram_selected && self.prg_ram_enabled {
                    let prg_ram_len = self.prg_ram.len();
                    self.prg_ram[((self.prg_banks[0] * 0x2000) + (address as usize - 0x6000)) % prg_ram_len] = data;
                }
            },
            0x8000 ..= 0x9FFF => {
                // Store the command to execute next
                self.command = data & 0b0000_1111;
            },
            0xA000 ..= 0xBFFF => {
                // Execute the stored command with the provided parameter byte
                match self.command {
                    0x0 ..= 0x7 => { self.chr_banks[self.command as usize] = data as usize},
                    0x8 =>  {
                        self.prg_ram_enabled = (data & 0b1000_0000) != 0;
                        self.prg_ram_selected = (data & 0b0100_0000) != 0;
                        self.prg_banks[0] = (data & 0b0011_1111) as usize;
                    },
                    0x9 ..= 0xB => {
                        self.prg_banks[(self.command - 0x8) as usize] = (data & 0b0011_1111) as usize;
                    },
                    0xC => {
                        match data & 0b0000_0011 {
                            0 => self.mirroring = Mirroring::Vertical,
                            1 => self.mirroring = Mirroring::Horizontal,
                            2 => self.mirroring = Mirroring::OneScreenLower,
                            3 => self.mirroring = Mirroring::OneScreenUpper,
                            _ => {}
                        }
                    },
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn write_ppu(&mut self, address: u16, data: u8) {
        match address {
            0x2000 ..= 0x3FFF => match self.mirroring {
                Mirroring::Horizontal => self.vram[mirroring::horizontal_mirroring(address) as usize] = data,
                Mirroring::Vertical   => self.vram[mirroring::vertical_mirroring(address) as usize] = data,
                Mirroring::OneScreenLower => self.vram[mirroring::one_screen_lower(address) as usize] = data,
                Mirroring::OneScreenUpper => self.vram[mirroring::one_screen_upper(address) as usize] = data,
                _ => {}
            },
            _ => {}
        }
    }
}
