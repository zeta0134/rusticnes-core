// A very simple Mapper with no esoteric features or bank switching.
// Reference capabilities: https://wiki.nesdev.com/w/index.php/NROM

use fds::FdsFile;

use mmc::mapper::*;
use mmc::mirroring;

pub struct FdsMapper {
    bios_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    chr: Vec<u8>,

    bios_loaded: bool,

    mirroring: Mirroring,
    vram: Vec<u8>,
}

impl FdsMapper {
    pub fn from_fds(_fds: FdsFile) -> Result<FdsMapper, String> {
        return Ok(FdsMapper {
            bios_rom: vec![0u8; 0x2000],
            prg_ram: vec![0u8; 0x8000],
            chr: vec![0u8; 0x2000],

            bios_loaded: false,

            mirroring: Mirroring::Horizontal,
            vram: vec![0u8; 0x1000],
        });
    }
}

impl Mapper for FdsMapper {
    fn print_debug_status(&self) {
        println!("======= FDS =======");
        println!("Mirroring Mode: {}", mirroring_mode_name(self.mirroring));
        println!("====================");
    }

    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }
    
    fn debug_read_cpu(&self, address: u16) -> Option<u8> {
        match address {
            0x6000 ..= 0xDFFF => {Some(self.prg_ram[address as usize - 0x6000])},
            0xE000 ..= 0xFFFF => {Some(self.bios_rom[address as usize - 0xE000])},
            _ => None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        match address {
            0x6000 ..= 0xDFFF => {self.prg_ram[address as usize - 0x6000] = data;},
            _ => {}
        }
    }

    fn debug_read_ppu(&self, address: u16) -> Option<u8> {
        match address {
            0x0000 ..= 0x1FFF => return Some(self.chr[address as usize]),
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
            0x0000 ..= 0x1FFF => {self.chr[address as usize] = data;},
            0x2000 ..= 0x3FFF => match self.mirroring {
                Mirroring::Horizontal => self.vram[mirroring::horizontal_mirroring(address) as usize] = data,
                Mirroring::Vertical   => self.vram[mirroring::vertical_mirroring(address) as usize] = data,
                _ => {}
            },
            _ => {}
        }
    }

    fn needs_bios(&self) -> bool {
        return !self.bios_loaded;
    }

    fn load_bios(&mut self, bios_rom: Vec<u8>) {
        self.bios_rom = bios_rom.clone();
        self.bios_loaded = true;
    }
}
