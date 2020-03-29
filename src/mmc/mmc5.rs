// Most powerful Nintendo produced mapper, supporting many advanced features
// As RusticNES doesn't support expansion audio, I'm not bothering to implement
// it here quite yet.
// Reference capabilities: https://wiki.nesdev.com/w/index.php/MMC5

use cartridge::NesHeader;
use mmc::mapper::*;
use std::cmp::min;
use std::cmp::max;

#[derive(Copy, Clone, PartialEq)]
pub enum PpuMode {
    Backgrounds,
    Sprites,
    PpuData
}

pub struct Mmc5 {
    pub prg_rom: Vec<u8>,
    pub prg_ram: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mirroring: Mirroring,
    pub ppuctrl_monitor: u8,
    pub ppumask_monitor: u8,
    pub prg_mode: u8,
    pub chr_mode: u8,
    pub prg_ram_magic_low: u8,
    pub prg_ram_magic_high: u8,
    pub extended_ram_mode: u8,
    pub vram: Vec<u8>,
    pub extram: Vec<u8>,
    pub nametable_mapping: u8,
    pub fill_tile: u8,
    pub fill_attr: u8,
    pub prg_bank_a_isram: bool,
    pub prg_bank_b_isram: bool,
    pub prg_bank_c_isram: bool,
    pub prg_bank_a: u8,
    pub prg_bank_b: u8,
    pub prg_bank_c: u8,
    pub prg_bank_d: u8,
    pub prg_ram_bank: u8,
    pub chr_banks: Vec<usize>,
    pub chr_ext_banks: Vec<usize>,
    pub chr_last_write_ext: bool,
    pub ppu_read_mode: PpuMode,
    pub chr_bank_high_bits: usize,
}

fn banked_memory_index(data_store_length: usize, bank_size: usize, bank_number: usize, raw_address: usize) -> usize {
    let total_banks = max(data_store_length / bank_size, 1);
    let selected_bank = bank_number % total_banks;
    let bank_start_offset = bank_size * selected_bank;
    let offset_within_bank = raw_address % min(bank_size, data_store_length);
    return bank_start_offset + offset_within_bank;
}

impl Mmc5 {
    pub fn new(header: NesHeader, chr: &[u8], prg: &[u8]) -> Mmc5 {
        let chr_rom = match header.has_chr_ram {
            true => vec![0u8; 8 * 1024],
            false => chr.to_vec()
        };

        return Mmc5 {
            prg_rom: prg.to_vec(),
            prg_ram: vec![0u8; 8 * 1024],
            chr_rom: chr_rom,
            mirroring: header.mirroring,
            ppuctrl_monitor: 0,
            ppumask_monitor: 0,
            prg_mode: 3,   // Koei games require MMC5 to boot into PRG mode 3
            chr_mode: 0,
            prg_ram_magic_low: 0,
            prg_ram_magic_high: 0,
            extended_ram_mode: 0,
            vram: vec![0u8; 0x1000],
            extram: vec![0u8; 0x800],
            nametable_mapping: 0,
            fill_tile: 0,
            fill_attr: 0,
            prg_bank_a: 0,
            prg_bank_b: 0,
            prg_bank_c: 0,
            prg_bank_d: 0x7F,   // Defaults to 0xFF, so interrupt vectors are loaded at boot
            prg_ram_bank: 0,
            prg_bank_a_isram: false,
            prg_bank_b_isram: false,
            prg_bank_c_isram: false,
            chr_banks: vec![0usize; 8],
            chr_ext_banks: vec![0usize; 8],
            chr_last_write_ext: false,
            ppu_read_mode: PpuMode::PpuData,
            chr_bank_high_bits: 0,
        }
    }

    pub fn large_sprites_active(&self) -> bool {
        return ((self.ppuctrl_monitor & 0b0010_0000) != 0) && ((self.ppumask_monitor & 0b0001_1000) != 0);
    }

    pub fn prg_ram_write_enabled(&self) -> bool {
        return (self.prg_ram_magic_low == 0b10) && (self.prg_ram_magic_high == 0b01);
    }

    // Nametable mapping helper functions, to assist with MMC5's arbitrary quadrant mapping
    pub fn nametable_vram_low(&self, address: u16) -> u8 {
        let masked_address = address & 0x3FF;
        return self.vram[masked_address as usize];
    }

    pub fn nametable_vram_high(&self, address: u16) -> u8 {
        let masked_address = address & 0x3FF;
        return self.vram[masked_address as usize + 0x400];
    }

    pub fn nametable_ext1(&self, address: u16) -> u8 {
        if self.extended_ram_mode == 0 || self.extended_ram_mode == 1 {
            let masked_address = address & 0x3FF;
            return self.extram[masked_address as usize];
        } else {
            return 0;
        }
    }

    pub fn nametable_fixed(&self, address: u16) -> u8 {
        let masked_address = address & 0x3FF;
        if masked_address < 0x3C0 {
            return self.fill_tile;
        } else {
            return self.fill_attr;
        }
    }

    pub fn read_nametable(&self, address: u16) -> u8 {
        let masked_address = address & 0xFFF;
        let quadrant = masked_address / 0x400;
        let nametable_select = (self.nametable_mapping >> quadrant * 2) & 0b11;
        return match nametable_select {
            0 => self.nametable_vram_low(masked_address),
            1 => self.nametable_vram_high(masked_address),
            2 => self.nametable_ext1(masked_address),
            3 => self.nametable_fixed(masked_address),
            _ => 0 // Shouldn't be reachable
        }
    }

    pub fn read_prg_mode_0(&self, address: u16) -> u8 {
        let (datastore, bank_number, bank_size) = match address {
            0x6000 ... 0x7FFF => (&self.prg_ram, self.prg_ram_bank, 8 * 1024),
            0x8000 ... 0xFFFF => (&self.prg_rom, self.prg_bank_d >> 2, 32 * 1024),
            _ => {return 0}
        };

        let datastore_offset = banked_memory_index(datastore.len(), bank_size, bank_number as usize, address as usize);
        return datastore[datastore_offset];
    }

    pub fn read_prg_mode_1(&self, address: u16) -> u8 {
        let (datastore, bank_number, bank_size) = match address {
            0x6000 ... 0x7FFF => (&self.prg_ram, self.prg_ram_bank, 8 * 1024),
            0x8000 ... 0xBFFF => match self.prg_bank_b_isram {
                true  => (&self.prg_ram, self.prg_bank_b >> 1, 16 * 1024),
                false => (&self.prg_rom, self.prg_bank_b >> 1, 16 * 1024)
            },
            0xC000 ... 0xFFFF => (&self.prg_rom, self.prg_bank_d >> 1, 16 * 1024),
            _ => {return 0}
        };

        let datastore_offset = banked_memory_index(datastore.len(), bank_size, bank_number as usize, address as usize);
        return datastore[datastore_offset];
    }

    pub fn read_prg_mode_2(&self, address: u16) -> u8 {
        let (datastore, bank_number, bank_size) = match address {
            0x6000 ... 0x7FFF => (&self.prg_ram, self.prg_ram_bank, 8 * 1024),
            0x8000 ... 0xBFFF => match self.prg_bank_b_isram {
                true  => (&self.prg_ram, self.prg_bank_b >> 1, 16 * 1024),
                false => (&self.prg_rom, self.prg_bank_b >> 1, 16 * 1024)
            },
            0xC000 ... 0xDFFF => match self.prg_bank_c_isram {
                true  => (&self.prg_ram, self.prg_bank_c, 8 * 1024),
                false => (&self.prg_rom, self.prg_bank_c, 8 * 1024)
            },
            0xE000 ... 0xFFFF => (&self.prg_rom, self.prg_bank_d, 8 * 1024),
            _ => {return 0}
        };

        let datastore_offset = banked_memory_index(datastore.len(), bank_size, bank_number as usize, address as usize);
        return datastore[datastore_offset];
    }

    pub fn read_prg_mode_3(&self, address: u16) -> u8 {
        let (datastore, bank_number, bank_size) = match address {
            0x6000 ... 0x7FFF => (&self.prg_ram, self.prg_ram_bank, 8 * 1024),
            0x8000 ... 0x9FFF => match self.prg_bank_a_isram {
                true  => (&self.prg_ram, self.prg_bank_a, 8 * 1024),
                false => (&self.prg_rom, self.prg_bank_a, 8 * 1024)
            },
            0xA000 ... 0xBFFF => match self.prg_bank_b_isram {
                true  => (&self.prg_ram, self.prg_bank_b, 8 * 1024),
                false => (&self.prg_rom, self.prg_bank_b, 8 * 1024)
            },
            0xC000 ... 0xDFFF => match self.prg_bank_c_isram {
                true  => (&self.prg_ram, self.prg_bank_c, 8 * 1024),
                false => (&self.prg_rom, self.prg_bank_c, 8 * 1024)
            },
            0xE000 ... 0xFFFF => (&self.prg_rom, self.prg_bank_d, 8 * 1024),
            _ => {return 0}
        };

        let datastore_offset = banked_memory_index(datastore.len(), bank_size, bank_number as usize, address as usize);
        return datastore[datastore_offset];
    }

    pub fn read_prg(&self, address: u16) -> u8 {
        return match self.prg_mode {
            0 => self.read_prg_mode_0(address),
            1 => self.read_prg_mode_1(address),
            2 => self.read_prg_mode_2(address),
            3 => self.read_prg_mode_3(address),
            _ => 0 // Should be unreachable
        }
    }

    pub fn write_prg_mode_0(&mut self, address: u16, data: u8) {
        let (bank_number, bank_size) = match address {
            0x6000 ... 0x7FFF => (self.prg_ram_bank, 8 * 1024),
            _ => {return}
        };

        let datastore_offset = banked_memory_index(self.prg_ram.len(), bank_size, bank_number as usize, address as usize);
        self.prg_ram[datastore_offset] = data;
    }

    pub fn write_prg_mode_1(&mut self, address: u16, data: u8) {
        let (bank_number, bank_size) = match address {
            0x6000 ... 0x7FFF => (self.prg_ram_bank, 8 * 1024),
            0x8000 ... 0xBFFF => match self.prg_bank_b_isram {
                true  => (self.prg_bank_b >> 1, 16 * 1024),
                false => {return}
            },
            _ => {return}
        };

        let datastore_offset = banked_memory_index(self.prg_ram.len(), bank_size, bank_number as usize, address as usize);
        self.prg_ram[datastore_offset] = data;
    }

    pub fn write_prg_mode_2(&mut self, address: u16, data: u8) {
        let (bank_number, bank_size) = match address {
            0x6000 ... 0x7FFF => (self.prg_ram_bank, 8 * 1024),
            0x8000 ... 0xBFFF => match self.prg_bank_b_isram {
                true  => (self.prg_bank_b >> 1, 16 * 1024),
                false => {return}
            },
            0xC000 ... 0xDFFF => match self.prg_bank_c_isram {
                true  => (self.prg_bank_c, 8 * 1024),
                false => {return}
            },
            _ => {return}
        };

        let datastore_offset = banked_memory_index(self.prg_ram.len(), bank_size, bank_number as usize, address as usize);
        self.prg_ram[datastore_offset] = data;
    }

    pub fn write_prg_mode_3(&mut self, address: u16, data: u8) {
        let (bank_number, bank_size) = match address {
            0x6000 ... 0x7FFF => (self.prg_ram_bank, 8 * 1024),
            0x8000 ... 0x9FFF => match self.prg_bank_a_isram {
                true  => (self.prg_bank_a, 8 * 1024),
                false => {return}
            },
            0xA000 ... 0xBFFF => match self.prg_bank_b_isram {
                true  => (self.prg_bank_b, 8 * 1024),
                false => {return}
            },
            0xC000 ... 0xDFFF => match self.prg_bank_c_isram {
                true  => (self.prg_bank_c, 8 * 1024),
                false => {return}
            },
            _ => {return}
        };

        let datastore_offset = banked_memory_index(self.prg_ram.len(), bank_size, bank_number as usize, address as usize);
        self.prg_ram[datastore_offset] = data;
    }

    pub fn write_prg(&mut self, address: u16, data: u8) {
        match self.prg_mode {
            0 => self.write_prg_mode_0(address, data),
            1 => self.write_prg_mode_1(address, data),
            2 => self.write_prg_mode_2(address, data),
            3 => self.write_prg_mode_3(address, data),
            _ => {} // Should be unreachable
        }
    }

    pub fn read_chr(&self, address: u16) -> u8 {
        let chr_bank_size = match self.chr_mode {
            0 => 8192,
            1 => 4096,
            2 => 2048,
            3 => 1024,
            _ => return 0
        };

        let chr_region = address / chr_bank_size;
        let standard_bank_index = (chr_region + 1) * (8 >> self.chr_mode) - 1;
        let extended_bank_index = standard_bank_index & 0x3;

        let large_sprites_enabled = self.ppuctrl_monitor & 0b0010_0000 != 0;
        let currently_reading_backgrounds = self.ppu_read_mode == PpuMode::Backgrounds;
        let ppu_inactive = self.ppu_read_mode == PpuMode::PpuData;
        let wrote_ext_register_last = self.chr_last_write_ext;

        if large_sprites_enabled && (currently_reading_backgrounds || (ppu_inactive && wrote_ext_register_last)) {
            let chr_bank = self.chr_ext_banks[extended_bank_index as usize];
            let chr_address = banked_memory_index(self.chr_rom.len(), chr_bank_size as usize, chr_bank, address as usize);
            return self.chr_rom[chr_address];
        } else {
            let chr_bank = self.chr_banks[standard_bank_index as usize];
            let chr_address = banked_memory_index(self.chr_rom.len(), chr_bank_size as usize, chr_bank, address as usize);
            return self.chr_rom[chr_address];
        }
    }
}

impl Mapper for Mmc5 {
    fn print_debug_status(&self) {
        println!("======= MMC5 =======");
        println!("PRG ROM: {}k, PRG RAM: {}k", self.prg_rom.len() / 1024, self.prg_ram.len() / 1024);
        println!("PRG Mode: {}", self.prg_mode);
        println!("PRG Banks: A:{} B:{} C:{} D:{} RAM:{}", self.prg_bank_a, self.prg_bank_b, self.prg_bank_c, self.prg_bank_d, self.prg_ram_bank);
        println!("====================");
    }

    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }
    
    fn read_cpu(&mut self, address: u16) -> Option<u8> {
        match address {
            0x5C00 ... 0x5FFF => {
                match self.extended_ram_mode {
                    2 ... 3 => {return Some(self.extram[address as usize - 0x5C00]);},
                    _ => return None
                }
            }
            0x6000 ... 0xFFFF => {return Some(self.read_prg(address))},
            _ => return None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        match address {
            0x2000 => {self.ppuctrl_monitor = data},
            0x2001 => {self.ppumask_monitor = data},
            0x5100 => {self.prg_mode = data & 0b0000_0011;},
            0x5101 => {self.chr_mode = data & 0b0000_0011;},
            0x5102 => {self.prg_ram_magic_low  = data & 0b0000_0011;},
            0x5103 => {self.prg_ram_magic_high = data & 0b0000_0011;},
            0x5104 => {self.extended_ram_mode = data & 0b0000_0011;},
            0x5105 => {self.nametable_mapping = data;},
            0x5106 => {self.fill_tile = data;},
            0x5107 => {
                let fill_color = data & 0b0000_0011;
                // For simplicity, go ahead and store the whole attribute byte
                self.fill_attr = (fill_color << 6) | (fill_color << 2) | (fill_color << 4) | (fill_color);
            },
            0x5113 => {self.prg_ram_bank = data & 0b0111_1111;},
            0x5114 => {
                self.prg_bank_a = data & 0b0111_1111;
                self.prg_bank_a_isram = (data & 0b1000_0000) == 0;
            },
            0x5115 => {
                self.prg_bank_b = data & 0b0111_1111;
                self.prg_bank_b_isram = (data & 0b1000_0000) == 0;
            },
            0x5116 => {
                self.prg_bank_c = data & 0b0111_1111;
                self.prg_bank_c_isram = (data & 0b1000_0000) == 0;
            },
            0x5117 => {self.prg_bank_d = data & 0b0111_1111;},
            0x5C00 ... 0x5FFF => {
                if self.extended_ram_mode == 2 {
                    self.extram[address as usize - 0x5C00] = data;
                }
            }
            0x5120 ... 0x5127 => {self.chr_banks[address as usize - 0x5120] = data as usize + self.chr_bank_high_bits;},
            0x5128 ... 0x512B => {self.chr_ext_banks[address as usize - 0x5128] = data as usize + self.chr_bank_high_bits;},
            0x5130 => {self.chr_bank_high_bits = (data as usize) << 8;},
            0x6000 ... 0xFFFF => {self.write_prg(address, data);},
            _ => {}
        }
    }

    fn read_ppu(&mut self, address: u16) -> Option<u8> {
        match address {
            0x0000 ... 0x1FFF => {return Some(self.read_chr(address))},
            0x2000 ... 0x3FFF => {return Some(self.read_nametable(address))},
            _ => return None
        }
    }

    fn write_ppu(&mut self, address: u16, data: u8) {
        match address {
            _ => {}
        }
    }
}

