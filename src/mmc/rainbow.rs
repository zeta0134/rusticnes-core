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
    Mode2Bank1x16k2x8k,
    Mode3Bank4x8k,
    Mode4Bank8x4k
}

pub enum PrgRamBankingMode {
    Mode0Bank1x8k,
    Mode1Bank2x4k
}

pub enum ChrBankingMode {
    Mode0Bank1x8k,
    Mode1Bank2x4k,
    Mode2Bank4x2k,
    Mode3Bank8x1k,
    Mode4Bank16x512b
}

pub enum ChrChipSelect {
    ChrRom,
    ChrRam,
    FpgaRam
}

pub struct Rainbow {
    prg_rom: MemoryBlock,
    prg_ram: MemoryBlock,
    chr_rom: MemoryBlock,
    chr_ram: MemoryBlock,

    prg_rom_mode: PrgRomBankingMode,
    prg_ram_mode: PrgRamBankingMode,
    chr_mode: ChrBankingMode,
    chr_chip: ChrChipSelect,

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

    fpga_bank_at_5000: usize,
    chr_banks: Vec<usize>,

    window_split: bool,
    extended_sprites: bool,

    mirroring: Mirroring,
    vram: Vec<u8>,
    fpga_ram: MemoryBlock,
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

        let mut fpga_ram: Vec<u8> = Vec::new();
        fpga_ram.resize(0x2000, 0);
        let fpga_ram_block = MemoryBlock::new(&fpga_ram, MemoryType::Ram);

        return Ok(Rainbow {
            prg_rom: prg_rom_block.clone(),
            prg_ram: prg_ram_block.clone(),
            chr_rom: chr_rom_block.clone(),
            chr_ram: chr_ram_block.clone(),

            prg_rom_mode: PrgRomBankingMode::Mode0Bank1x32k,
            prg_ram_mode: PrgRamBankingMode::Mode0Bank1x8k,
            chr_mode: ChrBankingMode::Mode0Bank1x8k,
            chr_chip: ChrChipSelect::ChrRom,

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

            fpga_bank_at_5000: 0,
            chr_banks: vec![0usize; 16],

            window_split: false,
            extended_sprites: false,

            mirroring: ines.header.mirroring(),
            vram: vec![0u8; 0x1000],
            fpga_ram: fpga_ram_block.clone(),
        });
    }

    // helper functions to deal with being able to selectively map ROM/RAM/FPGA into several regions
    fn read_banked_memory(&self, is_fpga: bool, is_ram: bool, bank_number: usize, blocksize: usize, address: usize) -> Option<u8> {
        if is_fpga {
            self.fpga_ram.banked_read(blocksize, bank_number, address)
        } else if is_ram {
            self.prg_ram.banked_read(blocksize, bank_number, address)
        } else {
            self.prg_rom.banked_read(blocksize, bank_number, address)
        }
    }

    fn write_banked_memory(&mut self, is_fpga: bool, is_ram: bool, bank_number: usize, blocksize: usize, address: usize, data: u8) {
        // Note: ignoring flash memory rewriting for now, that'll probably be handled elsewhere
        if is_fpga {
            self.fpga_ram.banked_write(blocksize, bank_number, address, data)
        } else if is_ram {
            self.prg_ram.banked_write(blocksize, bank_number, address, data)
        }
    }

    fn read_fpga_area(&self, address: usize) -> Option<u8> {
        self.read_banked_memory(true, false, self.fpga_bank_at_5000, 0x1000, address)
    }

    fn write_fpga_area(&mut self, address: usize, data: u8) {
        self.write_banked_memory(true, false, self.fpga_bank_at_5000, 0x1000, address, data)
    }

    fn read_prg_ram_area(&self, address: usize) -> Option<u8> {
        match self.prg_ram_mode {
            PrgRamBankingMode::Mode0Bank1x8k => self.read_banked_memory(self.fpga_ram_at_6000, self.prg_ram_at_6000, self.prg_bank_at_6000, 0x2000, address),
            PrgRamBankingMode::Mode1Bank2x4k => {
                match address {
                    0x6000 ..= 0x6FFF => self.read_banked_memory(self.fpga_ram_at_6000, self.prg_ram_at_6000, self.prg_bank_at_6000, 0x1000, address),
                    0x7000 ..= 0x7FFF => self.read_banked_memory(self.fpga_ram_at_7000, self.prg_ram_at_7000, self.prg_bank_at_7000, 0x1000, address),
                    _ => {None}
                }
            }
        }
    }

    fn write_prg_ram_area(&mut self, address: usize, data: u8) {
        match self.prg_ram_mode {
            PrgRamBankingMode::Mode0Bank1x8k => self.write_banked_memory(self.fpga_ram_at_6000, self.prg_ram_at_6000, self.prg_bank_at_6000, 0x2000, address, data),
            PrgRamBankingMode::Mode1Bank2x4k => {
                match address {
                    0x6000 ..= 0x6FFF => self.write_banked_memory(self.fpga_ram_at_6000, self.prg_ram_at_6000, self.prg_bank_at_6000, 0x1000, address, data),
                    0x7000 ..= 0x7FFF => self.write_banked_memory(self.fpga_ram_at_7000, self.prg_ram_at_7000, self.prg_bank_at_7000, 0x1000, address, data),
                    _ => {}
                }
            }
        }
    }

    fn read_prg_rom_area(&self, address: usize) -> Option<u8> {
        match self.prg_rom_mode {
            PrgRomBankingMode::Mode0Bank1x32k => self.read_banked_memory(false, self.prg_ram_at_8000, self.prg_bank_at_8000, 0x8000, address),
            PrgRomBankingMode::Mode1Bank2x16k => {
                match address {
                    0x8000 ..= 0xBFFF => self.read_banked_memory(false, self.prg_ram_at_8000, self.prg_bank_at_8000, 0x4000, address),
                    0xC000 ..= 0xFFFF => self.read_banked_memory(false, self.prg_ram_at_c000, self.prg_bank_at_c000, 0x4000, address),
                    _ => {None}
                }
            },
            PrgRomBankingMode::Mode2Bank1x16k2x8k => {
                match address {
                    0x8000 ..= 0xBFFF => self.read_banked_memory(false, self.prg_ram_at_8000, self.prg_bank_at_8000, 0x4000, address),
                    0xC000 ..= 0xDFFF => self.read_banked_memory(false, self.prg_ram_at_c000, self.prg_bank_at_c000, 0x2000, address),
                    0xE000 ..= 0xFFFF => self.read_banked_memory(false, self.prg_ram_at_e000, self.prg_bank_at_e000, 0x2000, address),
                    _ => {None}
                }
            },
            PrgRomBankingMode::Mode3Bank4x8k => {
                match address {
                    0x8000 ..= 0x9FFF => self.read_banked_memory(false, self.prg_ram_at_8000, self.prg_bank_at_8000, 0x2000, address),
                    0xA000 ..= 0xBFFF => self.read_banked_memory(false, self.prg_ram_at_a000, self.prg_bank_at_a000, 0x2000, address),
                    0xC000 ..= 0xDFFF => self.read_banked_memory(false, self.prg_ram_at_c000, self.prg_bank_at_c000, 0x2000, address),
                    0xE000 ..= 0xFFFF => self.read_banked_memory(false, self.prg_ram_at_e000, self.prg_bank_at_e000, 0x2000, address),
                    _ => {None}
                }
            },
            PrgRomBankingMode::Mode4Bank8x4k => {
                match address {
                    0x8000 ..= 0x8FFF => self.read_banked_memory(false, self.prg_ram_at_8000, self.prg_bank_at_8000, 0x1000, address),
                    0x9000 ..= 0x9FFF => self.read_banked_memory(false, self.prg_ram_at_9000, self.prg_bank_at_9000, 0x1000, address),
                    0xA000 ..= 0xAFFF => self.read_banked_memory(false, self.prg_ram_at_a000, self.prg_bank_at_a000, 0x1000, address),
                    0xB000 ..= 0xBFFF => self.read_banked_memory(false, self.prg_ram_at_b000, self.prg_bank_at_b000, 0x1000, address),
                    0xC000 ..= 0xCFFF => self.read_banked_memory(false, self.prg_ram_at_c000, self.prg_bank_at_c000, 0x1000, address),
                    0xD000 ..= 0xDFFF => self.read_banked_memory(false, self.prg_ram_at_d000, self.prg_bank_at_d000, 0x1000, address),
                    0xE000 ..= 0xEFFF => self.read_banked_memory(false, self.prg_ram_at_e000, self.prg_bank_at_e000, 0x1000, address),
                    0xF000 ..= 0xFFFF => self.read_banked_memory(false, self.prg_ram_at_f000, self.prg_bank_at_f000, 0x1000, address),
                    _ => {None}
                }
            },
        }
    }

    fn write_prg_rom_area(&mut self, address: usize, data: u8) {
        match self.prg_rom_mode {
            PrgRomBankingMode::Mode0Bank1x32k => self.write_banked_memory(false, self.prg_ram_at_8000, self.prg_bank_at_8000, 0x8000, address, data),
            PrgRomBankingMode::Mode1Bank2x16k => {
                match address {
                    0x8000 ..= 0xBFFF => self.write_banked_memory(false, self.prg_ram_at_8000, self.prg_bank_at_8000, 0x4000, address, data),
                    0xC000 ..= 0xFFFF => self.write_banked_memory(false, self.prg_ram_at_c000, self.prg_bank_at_c000, 0x4000, address, data),
                    _ => {}
                }
            },
            PrgRomBankingMode::Mode2Bank1x16k2x8k => {
                match address {
                    0x8000 ..= 0xBFFF => self.write_banked_memory(false, self.prg_ram_at_8000, self.prg_bank_at_8000, 0x4000, address, data),
                    0xC000 ..= 0xDFFF => self.write_banked_memory(false, self.prg_ram_at_c000, self.prg_bank_at_c000, 0x2000, address, data),
                    0xE000 ..= 0xFFFF => self.write_banked_memory(false, self.prg_ram_at_e000, self.prg_bank_at_e000, 0x2000, address, data),
                    _ => {}
                }
            },
            PrgRomBankingMode::Mode3Bank4x8k => {
                match address {
                    0x8000 ..= 0x9FFF => self.write_banked_memory(false, self.prg_ram_at_8000, self.prg_bank_at_8000, 0x2000, address, data),
                    0xA000 ..= 0xBFFF => self.write_banked_memory(false, self.prg_ram_at_a000, self.prg_bank_at_a000, 0x2000, address, data),
                    0xC000 ..= 0xDFFF => self.write_banked_memory(false, self.prg_ram_at_c000, self.prg_bank_at_c000, 0x2000, address, data),
                    0xE000 ..= 0xFFFF => self.write_banked_memory(false, self.prg_ram_at_e000, self.prg_bank_at_e000, 0x2000, address, data),
                    _ => {}
                }
            },
            PrgRomBankingMode::Mode4Bank8x4k => {
                match address {
                    0x8000 ..= 0x8FFF => self.write_banked_memory(false, self.prg_ram_at_8000, self.prg_bank_at_8000, 0x1000, address, data),
                    0x9000 ..= 0x9FFF => self.write_banked_memory(false, self.prg_ram_at_9000, self.prg_bank_at_9000, 0x1000, address, data),
                    0xA000 ..= 0xAFFF => self.write_banked_memory(false, self.prg_ram_at_a000, self.prg_bank_at_a000, 0x1000, address, data),
                    0xB000 ..= 0xBFFF => self.write_banked_memory(false, self.prg_ram_at_b000, self.prg_bank_at_b000, 0x1000, address, data),
                    0xC000 ..= 0xCFFF => self.write_banked_memory(false, self.prg_ram_at_c000, self.prg_bank_at_c000, 0x1000, address, data),
                    0xD000 ..= 0xDFFF => self.write_banked_memory(false, self.prg_ram_at_d000, self.prg_bank_at_d000, 0x1000, address, data),
                    0xE000 ..= 0xEFFF => self.write_banked_memory(false, self.prg_ram_at_e000, self.prg_bank_at_e000, 0x1000, address, data),
                    0xF000 ..= 0xFFFF => self.write_banked_memory(false, self.prg_ram_at_f000, self.prg_bank_at_f000, 0x1000, address, data),
                    _ => {}
                }
            },
        }
    }
}

                                

impl Mapper for Rainbow {
    fn print_debug_status(&self) {
        // TODO: ... do we even need this?
        println!("======= RAINBOW =======");        
        println!("====================");
    }

    fn mirroring(&self) -> Mirroring {
        // TODO: this is NROM! Fix this!
        return self.mirroring;
    }
    
    fn debug_read_cpu(&self, address: u16) -> Option<u8> {
        match address {
            // PRG banking modes
            0x4100 => {
                let prg_rom_mode_bits = match self.prg_rom_mode {
                    PrgRomBankingMode::Mode0Bank1x32k       => 0b000,
                    PrgRomBankingMode::Mode1Bank2x16k       => 0b001,
                    PrgRomBankingMode::Mode2Bank1x16k2x8k => 0b010,
                    PrgRomBankingMode::Mode3Bank4x8k        => 0b011,
                    // TODO: Does hardware preserve the low 2 bits written? If so we
                    // need an alternate approach when reading
                    PrgRomBankingMode::Mode4Bank8x4k        => 0b100
                };
                let prg_ram_mode_bits = match self.prg_ram_mode {
                    PrgRamBankingMode::Mode0Bank1x8k => 0b0000_0000,
                    PrgRamBankingMode::Mode1Bank2x4k => 0b1000_0000,
                };
                Some(prg_rom_mode_bits | prg_ram_mode_bits)
            },

            // CHR modes
            0x4120 => {
                let chr_banking_bits = match self.chr_mode {
                    ChrBankingMode::Mode0Bank1x8k => 0b000,
                    ChrBankingMode::Mode1Bank2x4k => 0b001,
                    ChrBankingMode::Mode2Bank4x2k => 0b010,
                    ChrBankingMode::Mode3Bank8x1k => 0b011,
                    ChrBankingMode::Mode4Bank16x512b => 0b100
                };
                let window_split_bit     = if self.window_split     {0b0001_0000} else {0};
                let extended_sprites_bit = if self.extended_sprites {0b0010_0000} else {0};
                let chip_select_bit = match self.chr_chip {
                    ChrChipSelect::ChrRom  => 0b0000_0000,
                    ChrChipSelect::ChrRam  => 0b0100_0000,
                    ChrChipSelect::FpgaRam => 0b1000_0000,
                };
                Some(chr_banking_bits | window_split_bit | extended_sprites_bit | chip_select_bit)
            },

            // mapper version
            0x4160 => {
                // 7  bit  0
                // ---- ----
                // PPPV VVVV
                // |||| ||||
                // |||+-++++- Version
                // +++------- Platform
                // Platform is one of:
                // 0   PCB
                // 1   Emulator
                // 2   Web emulator
                let platform = 0b001; // emulator
                let version  = 0;     // v1.0
                Some((platform << 5) | version)
            },

            0x4800 ..= 0x4FFF => self.fpga_ram.banked_read(0x800, 3, address as usize),
            0x5000 ..= 0x5FFF => self.read_fpga_area(address as usize),
            0x6000 ..= 0x7FFF => self.read_prg_ram_area(address as usize),
            0x8000 ..= 0xFFFF => self.read_prg_rom_area(address as usize),
            _ => None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        match address {
            // PRG banking modes
            0x4100 => {
                match data & 0b111 {
                    0b000 => self.prg_rom_mode = PrgRomBankingMode::Mode0Bank1x32k,
                    0b001 => self.prg_rom_mode = PrgRomBankingMode::Mode1Bank2x16k,
                    0b010 => self.prg_rom_mode = PrgRomBankingMode::Mode2Bank1x16k2x8k,
                    0b011 => self.prg_rom_mode = PrgRomBankingMode::Mode3Bank4x8k,
                        _ => self.prg_rom_mode = PrgRomBankingMode::Mode4Bank8x4k
                };
                match (data & 0b1000_0000) >> 7 {
                    0 => self.prg_ram_mode = PrgRamBankingMode::Mode0Bank1x8k,
                    1 => self.prg_ram_mode = PrgRamBankingMode::Mode1Bank2x4k,
                    _ => {/*unreachable*/}
                };
            }
            // PRG-RAM banking (upper)
            0x4106 => {
                match (data & 0b1100_0000) >> 6 {
                    0b00 ..= 0b01 => {
                        self.prg_ram_at_6000 = false;
                        self.fpga_ram_at_6000 = false;
                        self.prg_bank_at_6000 &= 0b1000_0000_1111_1111;
                        self.prg_bank_at_6000 |= ((data as usize) & 0b0111_1111) << 8
                    },
                    0b10 => {
                        self.prg_ram_at_6000 = true;
                        self.fpga_ram_at_6000 = false;
                        self.prg_bank_at_6000 &= 0b1100_0000_1111_1111;
                        self.prg_bank_at_6000 |= ((data as usize) & 0b0011_1111) << 8
                    },
                    0b11 => {
                        self.prg_ram_at_6000 = false;
                        self.fpga_ram_at_6000 = true;
                        self.prg_bank_at_6000 &= 0b1100_0000_1111_1111;
                        self.prg_bank_at_6000 |= ((data as usize) & 0b0011_1111) << 8
                    },
                    _ => {/*unreachable*/}
                }
            },
            0x4107 => {
                match (data & 0b1100_0000) >> 6 {
                    0b00 ..= 0b01 => {
                        self.prg_ram_at_7000 = false;
                        self.fpga_ram_at_7000 = false;
                        self.prg_bank_at_7000 &= 0b1000_0000_1111_1111;
                        self.prg_bank_at_7000 |= ((data as usize) & 0b0111_1111) << 8
                    },
                    0b10 => {
                        self.prg_ram_at_7000 = true;
                        self.fpga_ram_at_7000 = false;
                        self.prg_bank_at_7000 &= 0b1100_0000_1111_1111;
                        self.prg_bank_at_7000 |= ((data as usize) & 0b0011_1111) << 8
                    },
                    0b11 => {
                        self.prg_ram_at_7000 = false;
                        self.fpga_ram_at_7000 = true;
                        self.prg_bank_at_7000 &= 0b1100_0000_1111_1111;
                        self.prg_bank_at_7000 |= ((data as usize) & 0b0011_1111) << 8
                    },
                    _ => {/*unreachable*/}
                }
            },
            // PRG-ROM banking (upper)
            0x4108 => {
                self.prg_ram_at_8000  = (data & 0b1000_0000) != 0;
                self.prg_bank_at_8000 &= 0b1000_0000_1111_1111;
                self.prg_bank_at_8000 |= ((data as usize) & 0b0111_1111) << 8
            },
            0x4109 => {
                self.prg_ram_at_9000  = (data & 0b1000_0000) != 0;
                self.prg_bank_at_9000 &= 0b1000_0000_1111_1111;
                self.prg_bank_at_9000 |= ((data as usize) & 0b0111_1111) << 8
            },
            0x410A => {
                self.prg_ram_at_a000  = (data & 0b1000_0000) != 0;
                self.prg_bank_at_a000 &= 0b1000_0000_1111_1111;
                self.prg_bank_at_a000 |= ((data as usize) & 0b0111_1111) << 8
            },
            0x410B => {
                self.prg_ram_at_b000  = (data & 0b1000_0000) != 0;
                self.prg_bank_at_b000 &= 0b1000_0000_1111_1111;
                self.prg_bank_at_b000 |= ((data as usize) & 0b0111_1111) << 8
            },
            0x410C => {
                self.prg_ram_at_c000  = (data & 0b1000_0000) != 0;
                self.prg_bank_at_c000 &= 0b1000_0000_1111_1111;
                self.prg_bank_at_c000 |= ((data as usize) & 0b0111_1111) << 8
            },
            0x410D => {
                self.prg_ram_at_d000  = (data & 0b1000_0000) != 0;
                self.prg_bank_at_d000 &= 0b1000_0000_1111_1111;
                self.prg_bank_at_d000 |= ((data as usize) & 0b0111_1111) << 8
            },
            0x410E => {
                self.prg_ram_at_e000  = (data & 0b1000_0000) != 0;
                self.prg_bank_at_e000 &= 0b1000_0000_1111_1111;
                self.prg_bank_at_e000 |= ((data as usize) & 0b0111_1111) << 8
            },
            0x410F => {
                self.prg_ram_at_f000  = (data & 0b1000_0000) != 0;
                self.prg_bank_at_f000 &= 0b1000_0000_1111_1111;
                self.prg_bank_at_f000 |= ((data as usize) & 0b0111_1111) << 8
            },
            0x4115 => {
                self.fpga_bank_at_5000 = (data & 0b1) as usize;
            }
            // PRG-RAM banking (lower)
            0x4116 => {
                self.prg_bank_at_6000 &= 0b1111_1111_0000_0000;
                self.prg_bank_at_6000 |= data as usize;
            },
            0x4117 => {
                self.prg_bank_at_7000 &= 0b1111_1111_0000_0000;
                self.prg_bank_at_7000 |= data as usize;
            },
            // PRG-ROM banking (lower)
            0x4118 => {
                self.prg_bank_at_8000 &= 0b1111_1111_0000_0000;
                self.prg_bank_at_8000 |= data as usize
            },
            0x4119 => {
                self.prg_bank_at_9000 &= 0b1111_1111_0000_0000;
                self.prg_bank_at_9000 |= data as usize
            },
            0x411A => {
                self.prg_bank_at_a000 &= 0b1111_1111_0000_0000;
                self.prg_bank_at_a000 |= data as usize
            },
            0x411B => {
                self.prg_bank_at_b000 &= 0b1111_1111_0000_0000;
                self.prg_bank_at_b000 |= data as usize
            },
            0x411C => {
                self.prg_bank_at_c000 &= 0b1111_1111_0000_0000;
                self.prg_bank_at_c000 |= data as usize
            },
            0x411D => {
                self.prg_bank_at_d000 &= 0b1111_1111_0000_0000;
                self.prg_bank_at_d000 |= data as usize
            },
            0x411E => {
                self.prg_bank_at_e000 &= 0b1111_1111_0000_0000;
                self.prg_bank_at_e000 |= data as usize
            },
            0x411F => {
                self.prg_bank_at_f000 &= 0b1111_1111_0000_0000;
                self.prg_bank_at_f000 |= data as usize
            },
            0x4120 => {
                match data & 0b0000_0111 {
                    0b000 => self.chr_mode = ChrBankingMode::Mode0Bank1x8k,
                    0b001 => self.chr_mode = ChrBankingMode::Mode1Bank2x4k,
                    0b010 => self.chr_mode = ChrBankingMode::Mode2Bank4x2k,
                    0b011 => self.chr_mode = ChrBankingMode::Mode3Bank8x1k,
                    _ => self.chr_mode = ChrBankingMode::Mode4Bank16x512b,
                };
                self.window_split     = (data & 0b0001_0000) != 0;
                self.extended_sprites = (data & 0b0010_0000) != 0;
                match (data & 0b1100_0000) >> 6 {
                    0b00 => self.chr_chip = ChrChipSelect::ChrRom,
                    0b01 => self.chr_chip = ChrChipSelect::ChrRam,
                    _    => self.chr_chip = ChrChipSelect::FpgaRam
                };
            },

            0x4030 ..= 0x403F => {
                let bank_number = (address & 0x000F) as usize;
                self.chr_banks[bank_number] &= 0b0000_0000_1111_1111;
                self.chr_banks[bank_number] |= (data as usize) << 8;
            },
            0x4040 ..= 0x404F => {
                let bank_number = (address & 0x000F) as usize;
                self.chr_banks[bank_number] &= 0b1111_1111_0000_0000;
                self.chr_banks[bank_number] |= data as usize;
            },

            0x4800 ..= 0x4FFF => self.fpga_ram.banked_write(0x800, 3, address as usize, data),
            0x5000 ..= 0x5FFF => self.write_fpga_area(address as usize, data),
            0x6000 ..= 0x7FFF => self.write_prg_ram_area(address as usize, data),
            0x8000 ..= 0xFFFF => self.write_prg_rom_area(address as usize, data),

            _ => {}
        }
    }

    fn debug_read_ppu(&self, address: u16) -> Option<u8> {
        match address {
            // TODO: this is NROM! Fix this!
            0x0000 ..= 0x1FFF => return self.chr_rom.wrapping_read(address as usize),
            // TODO: this is NROM! Fix this!
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
            // TODO: this is NROM! Fix this!
            0x0000 ..= 0x1FFF => {self.chr_rom.wrapping_write(address as usize, data);},
            // TODO: this is NROM! Fix this!
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
