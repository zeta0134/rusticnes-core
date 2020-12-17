// Sunsoft FME-7, 5A, and 5B (notably lacking expansion audio for now)
// Reference implementation: https://wiki.nesdev.com/w/index.php/Sunsoft_FME-7

use cartridge::NesHeader;
use mmc::mapper::*;

pub struct Fme7 {
    pub command: u8,
    pub chr_banks: Vec<usize>,
    pub prg_banks: Vec<usize>,
    pub prg_ram_enabled: bool,
    pub prg_ram_selected: bool,
}

impl Fme7 {
    pub fn new(_: NesHeader, _: &[u8], _: &[u8]) -> Fme7 {
        return Fme7 {
            command: 0,
            chr_banks: vec![0usize; 8],
            prg_banks: vec![0usize; 4],
            prg_ram_enabled: false,
            prg_ram_selected: false,
        }
    }
}

impl Mapper for Fme7 {
    fn mirroring(&self) -> Mirroring {
        return Mirroring::Horizontal;
    }
    
    fn read_cpu(&mut self, _: u16) -> Option<u8> {
        return None;
    }

    fn read_ppu(&mut self, _: u16) -> Option<u8> {
        return None;
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        match address {
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
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn write_ppu(&mut self, _: u16, _: u8) {
        //Do nothing
    }    
}
