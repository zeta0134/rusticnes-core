// Most powerful Nintendo produced mapper, supporting many advanced features
// As RusticNES doesn't support expansion audio, I'm not bothering to implement
// it here quite yet.
// Reference capabilities: https://wiki.nesdev.com/w/index.php/MMC5

use cartridge::NesHeader;
use mmc::mapper::*;

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
    pub nametable_mapping: u8,
    pub fill_tile: u8,
    pub fill_attr: u8,
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
            prg_mode: 0,
            chr_mode: 0,
            prg_ram_magic_low: 0,
            prg_ram_magic_high: 0,
            extended_ram_mode: 0,
            vram: vec![0u8; 0x1800],
            nametable_mapping: 0,
            fill_tile: 0,
            fill_attr: 0,
        }
    }

    pub fn large_sprites_active(&self) -> bool {
        return ((self.ppuctrl_monitor & 0b0010_0000) != 0) && ((self.ppumask_monitor & 0b0001_1000) != 0);
    }

    pub fn prg_ram_write_enabled(&self) -> bool {
        return (self.prg_ram_magic_low == 0b10) && (self.prg_ram_magic_high == 0b01);
    }
}

impl Mapper for Mmc5 {
    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }
    
    fn read_cpu(&mut self, address: u16) -> Option<u8> {
        match address {
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
            _ => {}
        }
    }

    fn read_ppu(&mut self, address: u16) -> Option<u8> {
        match address {
            _ => return None
        }
    }

    fn write_ppu(&mut self, address: u16, data: u8) {
        match address {
            _ => {}
        }
    }
}
