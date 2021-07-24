// Namco 163 (and also 129), reference capabilities:
// https://wiki.nesdev.com/w/index.php?title=INES_Mapper_019

use ines::INesCartridge;
use memoryblock::MemoryBlock;
use memoryblock::MemoryType;

use mmc::mapper::*;

pub struct Namco163 {
    pub prg_rom: MemoryBlock,
    pub prg_ram: MemoryBlock,
    pub chr: MemoryBlock,
    pub vram: MemoryBlock,
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
            vram: MemoryBlock::new(&[0u8; 0x2000], MemoryType::Ram),
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

    pub fn read_banked_chr(&self, address: u16, bank_index: u8, use_nt: bool) -> Option<u8> {
        if use_nt {
            let effective_bank_index = bank_index & 0x1;
            return self.vram.banked_read(0x400, effective_bank_index as usize, address as usize);
        } else {
            return self.chr.banked_read(0x400, bank_index as usize, address as usize);
        }
    }
}

impl Mapper for Namco163 {
    fn mirroring(&self) -> Mirroring {
        return Mirroring::Horizontal;
    }
    
    fn debug_read_cpu(&self, _: u16) -> Option<u8> {
        return None;
    }

    fn debug_read_ppu(&self, address: u16) -> Option<u8> {
        let masked_address = address & 0xFC00;
        match masked_address {
            0x0000 => {self.read_banked_chr(address, self.chr_banks[0], self.nt_ram_at_0000)},
            0x0400 => {self.read_banked_chr(address, self.chr_banks[1], self.nt_ram_at_0000)},
            0x0800 => {self.read_banked_chr(address, self.chr_banks[2], self.nt_ram_at_0000)},
            0x0C00 => {self.read_banked_chr(address, self.chr_banks[3], self.nt_ram_at_0000)},
            0x1000 => {self.read_banked_chr(address, self.chr_banks[4], self.nt_ram_at_1000)},
            0x1400 => {self.read_banked_chr(address, self.chr_banks[5], self.nt_ram_at_1000)},
            0x1800 => {self.read_banked_chr(address, self.chr_banks[6], self.nt_ram_at_1000)},
            0x1C00 => {self.read_banked_chr(address, self.chr_banks[7], self.nt_ram_at_1000)},
            0x2000 => {self.read_banked_chr(address, self.nt_banks[0], true)},
            0x2400 => {self.read_banked_chr(address, self.nt_banks[1], true)},
            0x2800 => {self.read_banked_chr(address, self.nt_banks[2], true)},
            0x2C00 => {self.read_banked_chr(address, self.nt_banks[3], true)},
            _ => {None}
        }
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
            0x8000 => {self.chr_banks[0] = data;},
            0x8800 => {self.chr_banks[1] = data;},
            0x9000 => {self.chr_banks[2] = data;},
            0x9800 => {self.chr_banks[3] = data;},
            0xA000 => {self.chr_banks[4] = data;},
            0xA800 => {self.chr_banks[5] = data;},
            0xB000 => {self.chr_banks[6] = data;},
            0xB800 => {self.chr_banks[7] = data;},
            0xC000 => {self.nt_banks[0] = data;},
            0xC800 => {self.nt_banks[1] = data;},
            0xD000 => {self.nt_banks[2] = data;},
            0xD800 => {self.nt_banks[3] = data;},
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
