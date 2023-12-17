// A new homebrew mapper produced by Broke Studio. Used for the physical
// release of Super Tilt Bro. Currently in development, but the features
// should be mostly set in stone by this point. Documentation:
// https://github.com/BrokeStudio/rainbow-net/blob/master/NES/mapper-doc.md

// As the hardware is not yet released, this is a THEORETICAL mapper
// implementation. Once we have access to the hardware and can run real
// tests to verify behavior, the implementation will be updated and this
// notice removed. Until then, please be careful relying on this during
// new homebrew development.

use ines::INesCartridge;
use memoryblock::MemoryBlock;
use memoryblock::MemoryType;

use mmc::mapper::*;
use mmc::mirroring;

pub enum PrgRomBankingMode {
    Mode0Bank1x32k,
    Mode1Bank2x16k,
    Mode2Bank1x16k2x8k8k,
    Mode3Bank4x8k,
    Mode4Bank8x4k
}

pub enum PrgRamBankingMode {
    Mode0Bank1x8k,
    Mode1Bank2x4k
}

pub struct Rainbow {
    prg_rom: MemoryBlock,
    prg_ram: MemoryBlock,
    chr_rom: MemoryBlock,
    chr_ram: MemoryBlock,

    prg_rom_mode: PrgRomBankingMode,
    prg_ram_mode: PrgRamBankingMode,

    prg_bank_at_8000: usize,
    prg_bank_at_9000: usize,
    prg_bank_at_a000: usize,
    prg_bank_at_b000: usize,
    prg_bank_at_c000: usize,
    prg_bank_at_d000: usize,
    prg_bank_at_e000: usize,
    prg_bank_at_f000: usize,

    prg_ram_at_8000: bool,
    prg_ram_at_9000: bool,
    prg_ram_at_a000: bool,
    prg_ram_at_b000: bool,
    prg_ram_at_c000: bool,
    prg_ram_at_d000: bool,
    prg_ram_at_e000: bool,
    prg_ram_at_f000: bool,
    
    prg_bank_at_6000: usize,
    prg_bank_at_7000: usize,

    prg_ram_at_6000: bool,
    prg_ram_at_7000: bool,
    fpga_ram_at_6000: bool,
    fpga_ram_at_7000: bool,

    mirroring: Mirroring,
    vram: Vec<u8>,
    fpga_ram: Vec<u8>,
}

impl Rainbow {
    pub fn from_ines(ines: INesCartridge) -> Result<Rainbow, String> {
        // PRG ROM should always be present. We assume it is self-flashable
        // for emulation purposes.
        let prg_rom_block = ines.prg_rom_block();
        // PRG RAM follows the usual conventions: it may be battery
        // backed or not, but we don't support a mix of the two
        let prg_ram_block = ines.prg_ram_block()?;

        // CHR may be present in ROM, RAM, or a mix of the two, but will
        // never be battery backed. This is highly unusual among mappers,
        // so we'll parse the fields somewhat manually here. We assume
        // that CHR ROM is self-flashable.
        let chr_rom_block = if ines.chr.len() > 0 {
            MemoryBlock::new(&ines.chr, MemoryType::Rom)
        } else {
            MemoryBlock::new(&Vec::new(), MemoryType::Rom)
        };

        if ines.header.chr_ram_size() > 0 && ines.header.chr_sram_size() > 0 {
            return Err(format!("Rainbow: Unsupported mixed CHR types for mapper number {}", ines.header.mapper_number()));
        }

        let chr_ram_block = if ines.header.chr_ram_size() > 0 {
            let mut chr_ram: Vec<u8> = Vec::new();
            chr_ram.resize(ines.header.chr_ram_size(), 0);
            MemoryBlock::new(&chr_ram, MemoryType::Ram)
        } else if ines.header.chr_sram_size() > 0 {
            println!("Rainbow: Unsupported non-volatile CHR RAM! Loading anyway, will treat like volatile CHR RAM instead. Game saving may not work!");
            let mut chr_sram: Vec<u8> = Vec::new();
            chr_sram.resize(ines.header.chr_sram_size(), 0);
            MemoryBlock::new(&chr_sram, MemoryType::Ram)
        } else {
            MemoryBlock::new(&Vec::new(), MemoryType::Rom)
        };

        return Ok(Rainbow {
            prg_rom: prg_rom_block.clone(),
            prg_ram: prg_ram_block.clone(),
            chr_rom: chr_rom_block.clone(),
            chr_ram: chr_ram_block.clone(),

            prg_rom_mode: PrgRomBankingMode::Mode0Bank1x32k,
            prg_ram_mode: PrgRamBankingMode::Mode0Bank1x8k,

            prg_bank_at_8000: 0,
            prg_bank_at_9000: 0,
            prg_bank_at_a000: 0,
            prg_bank_at_b000: 0,
            prg_bank_at_c000: 0,
            prg_bank_at_d000: 0,
            prg_bank_at_e000: 0,
            prg_bank_at_f000: 0,

            prg_ram_at_8000: false,
            prg_ram_at_9000: false,
            prg_ram_at_a000: false,
            prg_ram_at_b000: false,
            prg_ram_at_c000: false,
            prg_ram_at_d000: false,
            prg_ram_at_e000: false,
            prg_ram_at_f000: false,
            
            prg_bank_at_6000: 0,
            prg_bank_at_7000: 0,

            prg_ram_at_6000: true,
            prg_ram_at_7000: true,
            fpga_ram_at_6000: false,
            fpga_ram_at_7000: false,

            mirroring: ines.header.mirroring(),
            vram: vec![0u8; 0x1000],
            fpga_ram: vec![0u8; 0x2000],
        });
    }
}

impl Mapper for Rainbow {
    fn print_debug_status(&self) {
        println!("======= RAINBOW =======");        
        println!("====================");
    }

    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }
    
    fn debug_read_cpu(&self, address: u16) -> Option<u8> {
        match address {
            0x6000 ..= 0x7FFF => {self.prg_ram.wrapping_read((address - 0x6000) as usize)},
            0x8000 ..= 0xFFFF => {self.prg_rom.wrapping_read((address - 0x8000) as usize)},
            _ => None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        match address {
            0x6000 ..= 0x7FFF => {self.prg_ram.wrapping_write((address - 0x6000) as usize, data);},
            _ => {}
        }
    }

    fn debug_read_ppu(&self, address: u16) -> Option<u8> {
        match address {
            0x0000 ..= 0x1FFF => return self.chr_rom.wrapping_read(address as usize),
            0x2000 ..= 0x3FFF => return match self.mirroring {
                Mirroring::Horizontal => Some(self.vram[mirroring::horizontal_mirroring(address) as usize]),
                Mirroring::Vertical   => Some(self.vram[mirroring::vertical_mirroring(address) as usize]),
                // Note: no licensed NROM boards support four-screen mirroring, but it is possible
                // to build a board that does. Since iNes allows this, some homebrew requires it, and
                // so we support it in the interest of compatibility.
                Mirroring::FourScreen => Some(self.vram[mirroring::four_banks(address) as usize]),
                _ => None
            },
            _ => return None
        }
    }

    fn write_ppu(&mut self, address: u16, data: u8) {
        match address {
            0x0000 ..= 0x1FFF => {self.chr_rom.wrapping_write(address as usize, data);},
            0x2000 ..= 0x3FFF => match self.mirroring {
                Mirroring::Horizontal => self.vram[mirroring::horizontal_mirroring(address) as usize] = data,
                Mirroring::Vertical   => self.vram[mirroring::vertical_mirroring(address) as usize] = data,
                Mirroring::FourScreen => self.vram[mirroring::four_banks(address) as usize] = data,
                _ => {}
            },
            _ => {}
        }
    }
}
