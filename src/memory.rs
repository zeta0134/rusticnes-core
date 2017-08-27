use nes::NesState;

pub struct CpuMemory {
    // Naive implementation -- a stupid array!
    //pub raw: [u8; 0x10000]
    pub iram_raw: [u8; 0x800],

    // Cartridge Space
    // TODO: Implement mappers. Not this.
    pub cart_ram: [u8; 0x2000],
    pub cart_rom: [u8; 0x8000],

    pub recent_reads: Vec<u16>,
    pub recent_writes: Vec<u16>,
}

impl CpuMemory {
    pub fn new() -> CpuMemory {
        return CpuMemory {
            iram_raw: [0u8; 0x800],
            cart_ram: [0u8; 0x2000],
            cart_rom: [0u8; 0x8000],
            recent_reads: Vec::new(),
            recent_writes: Vec::new(),
        }
    }
}

pub fn passively_read_byte(nes: &mut NesState, address: u16) -> u8 {
    return _read_byte(nes, address, false);
}

pub fn read_byte(nes: &mut NesState, address: u16) -> u8 {
    nes.memory.recent_reads.insert(0, address);
    nes.memory.recent_reads.truncate(20);
    return _read_byte(nes, address, true);
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
                        return (nes.ppu.status & 0xE0) + (nes.ppu.latch & 0x1F);
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
                        if nes.ppu.control & 0x04 == 0 {
                            nes.ppu.current_addr = nes.ppu.current_addr.wrapping_add(1);
                        } else {
                            nes.ppu.current_addr = nes.ppu.current_addr.wrapping_add(32);
                        }
                        return nes.ppu.latch;
                    } else {
                        return nes.ppu.read_byte(ppu_addr);
                    }
                },
                _ => return 0
            }
        },
        0x4016 => {
            if nes.input_latch {
                // strobe register is high, so copy input data to latch (probably bad if this
                // actually occurs here, but it matches what real hardware would do)
                nes.p1_data = nes.p1_input;
            }
            let result = 0x40 | (nes.p1_data & 0x1);
            nes.p1_data = nes.p1_data >> 1;
            return result;
        },
        0x4017 => {
            if nes.input_latch {
                // strobe register is high, so copy input data to latch (probably bad if this
                // actually occurs here, but it matches what real hardware would do)
                nes.p2_data = nes.p2_input;
            }
            let result = 0x40 | (nes.p2_data & 0x1);
            nes.p2_data = nes.p2_data >> 1;
            return result;
        },
        0x6000 ... 0x7FFF => return memory.cart_ram[(address & 0x1FFF) as usize],
        0x8000 ... 0xFFFF => return memory.cart_rom[(address & 0x7FFF) as usize],
        _ => return 0
    }
}

pub fn write_byte(nes: &mut NesState, address: u16, data: u8) {
    nes.memory.recent_writes.insert(0, address);
    nes.memory.recent_writes.truncate(20);
    match address {
        0x0000 ... 0x1FFF => nes.memory.iram_raw[(address & 0x7FF) as usize] = data,
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
                        nes.ppu.select_low = false;
                    } else {
                        nes.ppu.current_addr = (nes.ppu.current_addr & 0xFF) + ((data as u16) << 8);
                        nes.ppu.select_low = true;
                    }
                },
                // PPUDATA
                7 => {
                    let ppu_addr = nes.ppu.current_addr;
                    if nes.ppu.control & 0x04 == 0 {
                        nes.ppu.current_addr = nes.ppu.current_addr.wrapping_add(1);
                    } else {
                        nes.ppu.current_addr = nes.ppu.current_addr.wrapping_add(32);
                    }
                    nes.ppu.write_byte(ppu_addr, data);
                },
                _ => ()
            }
        },
        0x4014 => {
            // OAM DMA, for cheating just do this instantly and return
            let read_address = (data as u16) << 8;
            for i in 0 .. 256 {
                let byte = read_byte(nes, read_address + i);
                nes.ppu.oam[i as usize] = byte;
            }
        },
        0x4016 => {
            // Input latch
            nes.input_latch = data & 0x1 != 0;
            if nes.input_latch {
                nes.p1_data = nes.p1_input;
                nes.p2_data = nes.p2_input;
            }
        }
        0x6000 ... 0x7FFF => nes.memory.cart_ram[(address & 0x1FFF) as usize] = data,
        _ => () // Do nothing!
    }
}
