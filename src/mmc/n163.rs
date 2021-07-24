// Namco 163 (and also 129), reference capabilities:
// https://wiki.nesdev.com/w/index.php?title=INES_Mapper_019

use ines::INesCartridge;
use memoryblock::MemoryBlock;

use mmc::mapper::*;
use mmc::mirroring;

pub struct Namco163 {
}

impl Namco163 {
    pub fn from_ines(_ines: INesCartridge) -> Result<Namco163, String> {
        return Ok(Namco163 {
        })
    }
}

impl Mapper for Namco163 {
    fn mirroring(&self) -> Mirroring {
        return Mirroring::Horizontal;
    }
    
    fn debug_read_cpu(&self, _: u16) -> Option<u8> {
        return None;
    }

    fn debug_read_ppu(&self, _: u16) -> Option<u8> {
        return None;
    }

    fn write_cpu(&mut self, _: u16, _: u8) {
        //Do nothing
    }

    fn write_ppu(&mut self, _: u16, _: u8) {
        //Do nothing
    }    
}
