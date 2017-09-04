// A very simple Mapper with no esoteric features or bank switching.
// Reference capabilities: https://wiki.nesdev.com/w/index.php/NROM

use mmc::mapper::Mapper;

pub struct Nrom {
    pub prg_rom_size: usize,
    pub prg_rom: Vec<u8>,

    pub prg_ram_size: usize,
    pub prg_ram: Vec<u8>,

    pub chr_rom: Vec<u8>,
}

impl Mapper for Nrom {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000 ... 0x1FFF => return self.chr_rom[address as usize],
            0x6000 ... 0x7FFF => {
                if self.prg_ram_size > 0 {
                    return self.prg_ram[((address - 0x6000) % (self.prg_ram_size as u16)) as usize];
                } else {
                    return 0;
                }
            },
            0x8000 ... 0xFFFF => return self.prg_rom[(address % (self.prg_rom_size as u16)) as usize],
            _ => return 0
        }
    }

    fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            0x6000 ... 0x7FFF => {
                if self.prg_ram_size > 0 {
                    self.prg_ram[((address - 0x6000) % (self.prg_ram_size as u16)) as usize] = data;
                }
            },
            _ => {}
        }
    }
}
