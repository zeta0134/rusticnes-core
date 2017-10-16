// A dummy mapper with no loaded data. Useful for initializing an NesState
// with no actual cartridge loaded.

use mmc::mapper::*;

pub struct NoneMapper {
}

impl NoneMapper {
    pub fn new() -> NoneMapper {
        return NoneMapper {
        }
    }
}

impl Mapper for NoneMapper {
    fn mirroring(&self) -> Mirroring {
        return Mirroring::Horizontal;
    }
    
    fn read_byte(&mut self, _: u16) -> u8 {
        return 0;
    }

    fn write_byte(&mut self, _: u16, _: u8) {
        //Do nothing
    }
}
