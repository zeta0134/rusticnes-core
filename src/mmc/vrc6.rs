// Vrc6, 
// Reference capabilities: https://wiki.nesdev.com/w/index.php/VRC6

use cartridge::NesHeader;
use mmc::mapper::*;

pub struct Vrc6 {
    pub prg_rom: Vec<u8>,
    pub prg_ram: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub vram: Vec<u8>,
    pub prg_ram_enable: bool,
    pub prg_bank_16: usize,
    pub prg_bank_8: usize,
    pub r: Vec<usize>,
    pub ppu_banking_mode: u8,
    pub mirroring_mode: u8,
    pub nametable_chrrom: bool,
    pub chr_a10_rules: bool,
    pub mirroring: Mirroring,
}

impl Vrc6 {
    pub fn new(header: NesHeader, chr: &[u8], prg: &[u8]) -> Vrc6 {
        return Vrc6 {
            prg_rom: prg.to_vec(),
            prg_ram: vec![0u8; 8 * 1024],
            chr_rom: chr.to_vec(),
            vram: vec![0u8; 0x1000],
            prg_ram_enable: false,
            prg_bank_16: 0,
            prg_bank_8: 0,
            r: vec![0usize; 8],
            ppu_banking_mode: 0,
            mirroring_mode: 0,
            nametable_chrrom: false,
            chr_a10_rules: false,
            mirroring: header.mirroring,
        }
    }

    fn _chr_mode_0(&self, address: u16) -> u8 {
        // All 8k banks
        let chr_rom_len = self.chr_rom.len();
        match address {
            0x0000 ..= 0x03FF => return self.chr_rom[((self.r[0] * 0x400) + (address as usize -  0x0000)) % chr_rom_len],
            0x0400 ..= 0x07FF => return self.chr_rom[((self.r[1] * 0x400) + (address as usize -  0x0400)) % chr_rom_len],
            0x0800 ..= 0x0BFF => return self.chr_rom[((self.r[2] * 0x400) + (address as usize -  0x0800)) % chr_rom_len],
            0x0C00 ..= 0x0FFF => return self.chr_rom[((self.r[3] * 0x400) + (address as usize -  0x0C00)) % chr_rom_len],
            0x1000 ..= 0x13FF => return self.chr_rom[((self.r[4] * 0x400) + (address as usize -  0x1000)) % chr_rom_len],
            0x1400 ..= 0x17FF => return self.chr_rom[((self.r[5] * 0x400) + (address as usize -  0x1400)) % chr_rom_len],
            0x1800 ..= 0x1BFF => return self.chr_rom[((self.r[6] * 0x400) + (address as usize -  0x1800)) % chr_rom_len],
            0x1C00 ..= 0x1FFF => return self.chr_rom[((self.r[7] * 0x400) + (address as usize -  0x1C00)) % chr_rom_len],
            _ => return 0 // never reached
        }
    }

    fn _chr_mode_1(&self, address: u16) -> u8 {
        // All 16k banks, with differing A10 behavior
        let chr_rom_len = self.chr_rom.len();
        if self.chr_a10_rules {
            //16kb banks use PPU A10, ignore low bit of register
            match address {
                0x0000 ..= 0x07FF => return self.chr_rom[(((self.r[0] & 0xFE) * 0x400) + (address as usize -  0x0000)) % chr_rom_len],
                0x0800 ..= 0x0FFF => return self.chr_rom[(((self.r[1] & 0xFE) * 0x400) + (address as usize -  0x0800)) % chr_rom_len],
                0x1000 ..= 0x17FF => return self.chr_rom[(((self.r[2] & 0xFE) * 0x400) + (address as usize -  0x1000)) % chr_rom_len],
                0x1800 ..= 0x1FFF => return self.chr_rom[(((self.r[3] & 0xFE) * 0x400) + (address as usize -  0x1800)) % chr_rom_len],
                _ => return 0 // never reached
            }
        } else {
            // Low bit of register determines A10, effectively duplicating 1k banks, similar to 8k mode
            match address {
                0x0000 ..= 0x03FF => return self.chr_rom[((self.r[0] * 0x400) + (address as usize -  0x0000)) % chr_rom_len],
                0x0400 ..= 0x07FF => return self.chr_rom[((self.r[0] * 0x400) + (address as usize -  0x0400)) % chr_rom_len],
                0x0800 ..= 0x0BFF => return self.chr_rom[((self.r[1] * 0x400) + (address as usize -  0x0800)) % chr_rom_len],
                0x0C00 ..= 0x0FFF => return self.chr_rom[((self.r[1] * 0x400) + (address as usize -  0x0C00)) % chr_rom_len],
                0x1000 ..= 0x13FF => return self.chr_rom[((self.r[2] * 0x400) + (address as usize -  0x1000)) % chr_rom_len],
                0x1400 ..= 0x17FF => return self.chr_rom[((self.r[2] * 0x400) + (address as usize -  0x1400)) % chr_rom_len],
                0x1800 ..= 0x1BFF => return self.chr_rom[((self.r[3] * 0x400) + (address as usize -  0x1800)) % chr_rom_len],
                0x1C00 ..= 0x1FFF => return self.chr_rom[((self.r[3] * 0x400) + (address as usize -  0x1C00)) % chr_rom_len],
                _ => return 0 // never reached
            }
        }
    }

    fn _chr_mode_23(&self, address: u16) -> u8 {
        // Essentially a mix, mode 0 for the upper half, with 2 16k banks in the lower half that behave similarly to mode 1
        // but pull from R4-R5 instead
        let chr_rom_len = self.chr_rom.len();
        match address {
            0x0000 ..= 0x0FFF => return self._chr_mode_0(address),
            0x1000 ..= 0x1FFF => {
                if self.chr_a10_rules {
                    //16kb banks use PPU A10, ignore low bit of register
                    match address {
                        0x1000 ..= 0x17FF => return self.chr_rom[(((self.r[4] & 0xFE) * 0x400) + (address as usize -  0x1000)) % chr_rom_len],
                        0x1800 ..= 0x1FFF => return self.chr_rom[(((self.r[5] & 0xFE) * 0x400) + (address as usize -  0x1800)) % chr_rom_len],
                        _ => return 0 // never reached
                    }
                } else {
                    // Low bit of register determines A10, effectively duplicating 1k banks, similar to 8k mode
                    match address {
                        0x1000 ..= 0x13FF => return self.chr_rom[((self.r[4] * 0x400) + (address as usize -  0x1000)) % chr_rom_len],
                        0x1400 ..= 0x17FF => return self.chr_rom[((self.r[4] * 0x400) + (address as usize -  0x1400)) % chr_rom_len],
                        0x1800 ..= 0x1BFF => return self.chr_rom[((self.r[5] * 0x400) + (address as usize -  0x1800)) % chr_rom_len],
                        0x1C00 ..= 0x1FFF => return self.chr_rom[((self.r[5] * 0x400) + (address as usize -  0x1C00)) % chr_rom_len],
                        _ => return 0 // never reached
                    }
                }
            }
            _ => return 0 // never reached
        }
    }
}

impl Mapper for Vrc6 {
    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }

    fn read_cpu(&mut self, address: u16) -> Option<u8> {
        let prg_rom_len = self.prg_rom.len();
        match address {
            0x6000 ..= 0x7FFF => {
                return Some(self.prg_ram[(address - 0x6000) as usize]);
            },
            0x8000 ..= 0xBFFF => {
                return Some(self.prg_rom[((self.prg_bank_16 * 0x4000) + (address as usize - 0x8000)) % prg_rom_len]);
            },
            0xC000 ..= 0xDFFF => {
                return Some(self.prg_rom[((self.prg_bank_8 * 0x2000) + (address as usize - 0xC000)) % prg_rom_len]);
            },
            0xE000 ..= 0xFFFF => {
                return Some(self.prg_rom[(prg_rom_len - 0x2000) + (address as usize -  0xE000)]);
            },
            _ => return None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        match address {
            0x6000 ..= 0x7FFF => {
                if self.prg_ram_enable {
                    self.prg_ram[(address - 0x6000) as usize] = data;
                }
            },
            _ => {}
        }
        let masked_address = address & 0b1111_0000_0000_0011;
        match masked_address {
            0x8000 ..= 0x8003 => {
                self.prg_bank_16 = data as usize & 0x0F;
            },
            0xB003 => {
                self.ppu_banking_mode = data & 0b0000_0011;
                self.mirroring_mode = (data & 0b0000_1100) >> 2;
                self.nametable_chrrom = (data & 0b0001_0000) != 0;
                self.chr_a10_rules = (data & 0b0010_0000) != 0;
                self.prg_ram_enable = (data & 0b1000_0000) != 0;

                println!("PPU Banking Mode: {}, CHR A10: {}", self.ppu_banking_mode, self.chr_a10_rules);
                println!("Mirroring Mode: {}, Nametable CHR ROM: {}", self.mirroring_mode, self.nametable_chrrom);

            },
            0xC000 ..= 0xC003 => {
                self.prg_bank_8 = data as usize & 0x1F;
            },
            0xD000 => { self.r[0] = data as usize; },
            0xD001 => { self.r[1] = data as usize; },
            0xD002 => { self.r[2] = data as usize; },
            0xD003 => { self.r[3] = data as usize; },
            0xE000 => { self.r[4] = data as usize; },
            0xE001 => { self.r[5] = data as usize; },
            0xE002 => { self.r[6] = data as usize; },
            0xE003 => { self.r[7] = data as usize; },
            _ => {}
        }
    }

    fn read_ppu(&mut self, address: u16) -> Option<u8> {
        match address {
            0x0000 ..= 0x1FFF => {
                // CHR Bank Selection
                match self.ppu_banking_mode {
                    0 => return Some(self._chr_mode_0(address)),
                    1 => return Some(self._chr_mode_1(address)),
                    2 => return Some(self._chr_mode_23(address)),
                    3 => return Some(self._chr_mode_23(address)),
                    _ => return None
                }
            }
            _ => return None
        }
    }

    fn write_ppu(&mut self, address: u16, data: u8) {
        match address {
            _ => {}
        }
    }
}