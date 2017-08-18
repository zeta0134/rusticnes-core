use std::ops::Index;
use std::ops::IndexMut;

use nes::NesState;

pub struct CpuMemory {
    // Naive implementation -- a stupid array!
    //pub raw: [u8; 0x10000]
    pub iram_raw: [u8; 0x800],

    // Cartridge Space
    // TODO: Implement mappers. Not this.
    pub cart_rom: [u8; 0x8000],
}

impl CpuMemory {
    pub fn new() -> CpuMemory {
        return CpuMemory {
            iram_raw: [0u8; 0x800],
            cart_rom: [0u8; 0x8000],
        }
    }
}

pub fn passively_read_byte(state: &mut NesState, address: u16) -> u8 {
    return _read_byte(state, address, false);
}

pub fn read_byte(state: &mut NesState, address: u16) -> u8 {
    return _read_byte(state, address, true);
}

fn _read_byte(nes: &mut NesState, address: u16, side_effects: bool) -> u8 {
    let memory = &mut nes.memory;
    match address {
        0x0000 ... 0x1FFF => return memory.iram_raw[(address & 0x7FF) as usize],
        0x2000 ... 0x3FFF => {
            // PPU
            let ppu_reg = address & 0x7;
            match ppu_reg {
                // PPUCTRL, PPUMASK, OAMADDR | PPUSCROLL | PPUADDR (Write Only)
                0 | 1 | 3 | 5 | 6 => {
                    return nes.ppu.latch;
                },
                // PPUSTATUS
                2 => {
                    if side_effects {
                        nes.ppu.select_scroll_y = false;
                        nes.ppu.select_low = false;
                        nes.ppu.latch = (nes.ppu.status & 0xE0) + (nes.ppu.latch & 0x1F);
                        nes.ppu.status = nes.ppu.status & 0x7F; // Clear VBlank bit
                        return nes.ppu.latch;
                    } else {
                        return nes.ppu.status & 0xE0 + nes.ppu.latch & 0x1F;
                    }
                },
                // OAMDATA
                4 => {
                    if side_effects {
                        nes.ppu.latch = nes.ppu.oam[nes.ppu.oam_addr as usize];
                        return nes.ppu.latch;
                    } else {
                        return nes.ppu.oam[nes.ppu.oam_addr as usize];
                    }
                },
                // PPUDATA
                7 => {
                    let ppu_addr = nes.ppu.current_addr;
                    if side_effects {
                        nes.ppu.latch = nes.ppu.read_byte(ppu_addr);
                        return nes.ppu.latch;
                    } else {
                        return nes.ppu.read_byte(ppu_addr);
                    }
                },
                _ => return 0
            }
        },
        0x8000 ... 0xFFFF => return memory.cart_rom[(address & 0x7FFF) as usize],
        _ => return 0
    }
}

pub fn write_byte(nes: &mut NesState, address: u16, data: u8) {
    let memory = &mut nes.memory;
    match address {
        0x0000 ... 0x1FFF => memory.iram_raw[(address & 0x7FF) as usize] = data,
        0x2000 ... 0x3FFF => {
            // PPU
            let ppu_reg = address & 0x7;
            nes.ppu.latch = data;
            match ppu_reg {
                // PPUCTRL
                0 => {
                    nes.ppu.control = data;
                },
                // PPU MASK
                1 => {
                    nes.ppu.mask = data;
                },
                // PPUSTATUS
                2 => {
                    nes.ppu.status = data & 0xE0;
                },
                // OAM ADDRESS
                3 => {
                    nes.ppu.oam_addr = data;
                },
                // OAMDATA
                4 => {
                    nes.ppu.oam[nes.ppu.oam_addr as usize] = data;
                },
                // PPU SCROLL
                5 => {
                    if nes.ppu.select_scroll_y {
                        nes.ppu.scroll_y = data;
                        nes.ppu.select_scroll_y = false;
                    } else {
                        nes.ppu.scroll_x = data;
                        nes.ppu.select_scroll_y = true;
                    }
                },
                // PPU ADDR
                6 => {
                    if nes.ppu.select_low {
                        nes.ppu.current_addr = (nes.ppu.current_addr & 0xFF00) + data as u16;
                        nes.ppu.select_scroll_y = false;
                    } else {
                        nes.ppu.current_addr = (nes.ppu.current_addr & 0xFF) + ((data as u16) << 8);
                        nes.ppu.select_low = true;
                    }
                },
                // PPUDATA
                7 => {
                    let ppu_addr = nes.ppu.current_addr;
                    nes.ppu.write_byte(ppu_addr, data);
                },
                _ => ()
            }
        }
        _ => () // Do nothing!
    }
}
