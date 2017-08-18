use std::ops::Index;
use std::ops::IndexMut;

use nes::NesState;

pub struct CpuMemory {
    // Naive implementation -- a stupid array!
    //pub raw: [u8; 0x10000]
    pub iram_raw: [u8; 0x800],

    // PPU Registers
    ppu_latch: u8,

    pub ppu_control: u8,
    pub ppu_mask: u8,
    pub ppu_status: u8,
    pub oam_addr: u8,

    // OAM (sprite memory)

    // Scrolling, which is implemented with a flip/flop register
    pub ppu_select_scroll_y: bool,
    pub ppu_scroll_x: u8,
    pub ppu_scroll_y: u8,

    // PPU Address, similar to scrolling, has a high / low component
    pub ppu_select_low: bool,
    pub ppu_addr: u16,

    pub oam_dma_high: u8,
    // APU Registers (HAHA, Later)

    // Cartridge Space
    // TODO: Implement mappers. Not this.
    pub cart_rom: [u8; 0x8000],
}

impl CpuMemory {
    pub fn new() -> CpuMemory {
        return CpuMemory {
            iram_raw: [0u8; 0x800],
            cart_rom: [0u8; 0x8000],
            ppu_control: 0,
            ppu_mask: 0,
            ppu_status: 0,
            oam_addr: 0,
            ppu_select_scroll_y: false,
            ppu_scroll_x: 0,
            ppu_scroll_y: 0,
            ppu_select_low: false,
            ppu_addr: 0,
            oam_dma_high: 0,
            ppu_latch: 0,
        }
    }
}

pub fn passively_read_byte(state: &mut NesState, address: u16) -> u8 {
    return _read_byte(state, address, false);
}

pub fn read_byte(state: &mut NesState, address: u16) -> u8 {
    return _read_byte(state, address, true);
}

fn _read_byte(state: &mut NesState, address: u16, side_effects: bool) -> u8 {
    let memory = &mut state.memory;
    match address {
        0x0000 ... 0x1FFF => return memory.iram_raw[(address & 0x7FF) as usize],
        0x2000 ... 0x3FFF => {
            // PPU
            let ppu_reg = address & 0x7;
            println!("PPU Register Read: {}", ppu_reg);
            match ppu_reg {
                // PPUCTRL, PPUMASK, OAMADDR | PPUSCROLL | PPUADDR (Write Only)
                0 | 1 | 3 | 5 | 6 => {
                    return memory.ppu_latch;
                },
                // PPUSTATUS
                2 => {
                    if side_effects {
                        println!("Status Register Read: {}", memory.ppu_status);
                        memory.ppu_select_scroll_y = false;
                        memory.ppu_select_low = false;
                        memory.ppu_latch = (memory.ppu_status & 0xE0) + (memory.ppu_latch & 0x1F);
                        memory.ppu_status = memory.ppu_status & 0x7F; // Clear VBlank bit
                        return memory.ppu_latch;
                    } else {
                        return memory.ppu_status & 0xE0 + memory.ppu_latch & 0x1F;
                    }
                },
                // OAMDATA
                4 => {
                    if side_effects {
                        memory.ppu_latch = state.ppu.oam[memory.oam_addr as usize];
                        return memory.ppu_latch;
                    } else {
                        return state.ppu.oam[memory.oam_addr as usize];
                    }
                },
                // PPUDATA
                7 => {
                    if side_effects {
                        memory.ppu_latch = state.ppu.read_byte(memory.ppu_addr);
                        return memory.ppu_latch;
                    } else {
                        return state.ppu.read_byte(memory.ppu_addr);
                    }
                },
                _ => return 0
            }
        },
        0x8000 ... 0xFFFF => return memory.cart_rom[(address & 0x7FFF) as usize],
        _ => return 0
    }
}

pub fn write_byte(state: &mut NesState, address: u16, data: u8) {
    let memory = &mut state.memory;
    match address {
        0x0000 ... 0x1FFF => memory.iram_raw[(address & 0x7FF) as usize] = data,
        0x2000 ... 0x3FFF => {
            // PPU
            let ppu_reg = address & 0x7;
            memory.ppu_latch = data;
            match ppu_reg {
                // PPUCTRL
                0 => {
                    memory.ppu_control = data;
                },
                // PPU MASK
                1 => {
                    memory.ppu_mask = data;
                },
                // PPUSTATUS
                2 => {
                    memory.ppu_status = data & 0xE0;
                },
                // OAM ADDRESS
                3 => {
                    memory.oam_addr = data;
                },
                // OAMDATA
                4 => {
                    state.ppu.oam[memory.oam_addr as usize] = data;
                },
                // PPU SCROLL
                5 => {
                    if memory.ppu_select_scroll_y {
                        memory.ppu_scroll_y = data;
                        memory.ppu_select_scroll_y = false;
                    } else {
                        memory.ppu_scroll_x = data;
                        memory.ppu_select_scroll_y = true;
                    }
                },
                // PPU ADDR
                6 => {
                    if memory.ppu_select_low {
                        memory.ppu_addr = (memory.ppu_addr & 0xFF00) + data as u16;
                        memory.ppu_select_scroll_y = false;
                    } else {
                        memory.ppu_addr = (memory.ppu_addr & 0xFF) + ((data as u16) << 8);
                        memory.ppu_select_low = true;
                    }
                },
                // PPUDATA
                7 => {
                    state.ppu.write_byte(memory.ppu_addr, data);
                },
                _ => ()
            }
        }
        _ => () // Do nothing!
    }
}
