// Namco 163 (and also 129), reference capabilities:
// https://wiki.nesdev.com/w/index.php?title=INES_Mapper_019

use ines::INesCartridge;
use memoryblock::MemoryBlock;

use mmc::mapper::*;

pub struct Namco163 {
    pub prg_rom: MemoryBlock,
    pub prg_ram: MemoryBlock,
    pub chr: MemoryBlock,
    pub vram: Vec<u8>,
    pub internal_ram: Vec<u8>,

    pub irq_enabled: bool,
    pub irq_pending: bool,
    pub irq_counter: u16, // 15bit, actually

    pub chr_banks: Vec<u8>,
    pub nt_banks: Vec<u8>,
    pub prg_banks: Vec<u8>,

    pub write_enable: u8,
    pub sound_disable: bool,
    pub nt_ram_at_0000: bool,
    pub nt_ram_at_1000: bool,

}

impl Namco163 {
    pub fn from_ines(ines: INesCartridge) -> Result<Namco163, String> {
        let prg_rom_block = ines.prg_rom_block();
        let prg_ram_block = ines.prg_ram_block()?;
        let chr_block = ines.chr_block()?;

        return Ok(Namco163 {
            prg_rom: prg_rom_block.clone(),
            prg_ram: prg_ram_block.clone(),
            chr: chr_block.clone(),
            vram: vec![0u8; 0x2000],
            internal_ram: vec![0u8; 0x80],

            irq_enabled: false,
            irq_pending: false,
            irq_counter: 0,

            chr_banks: vec![0u8; 8],
            nt_banks: vec![0u8; 4],
            prg_banks: vec![0u8; 3],

            write_enable: 0, // upper nybble mismatch, will disable PRG RAM at boot
            sound_disable: true,
            nt_ram_at_0000: false,
            nt_ram_at_1000: false,
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

    fn write_cpu(&mut self, address: u16, data: u8) {
        let masked_address = address & 0xF800;
        match masked_address {
            0x5000 => {
                let irq_low = data as u16;
                self.irq_counter = (self.irq_counter & 0xFF00) | irq_low;
                self.irq_pending = false;
            },
            0x5800 => {
                let irq_high = ((data as u16) & 0x7F) << 8;
                self.irq_counter = (self.irq_counter & 0x00FF) | irq_high;
                self.irq_enabled = (data & 0x80) != 0;
                self.irq_pending = false;
            },
            _ => {}
        }
    }

    fn clock_cpu(&mut self) {
        if self.irq_enabled && self.irq_counter < 0x7FFF {
            self.irq_counter += 1;
            if self.irq_counter == 0x7FFF {
                self.irq_pending = true;
            }
        }
    }

    fn irq_flag(&self) -> bool {
        return self.irq_pending;
    }

    fn write_ppu(&mut self, _: u16, _: u8) {
        //Do nothing
    }    
}
