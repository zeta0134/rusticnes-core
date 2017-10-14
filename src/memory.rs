use nes::NesState;

pub struct CpuMemory {
    pub iram_raw: Vec<u8>,

    pub recent_reads: Vec<u16>,
    pub recent_writes: Vec<u16>,
}

impl CpuMemory {
    pub fn new() -> CpuMemory {
        return CpuMemory {
            iram_raw: vec!(0u8; 0x800),
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
                        nes.ppu.write_toggle = false;
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
                    let ppu_addr = nes.ppu.current_vram_address;
                    if side_effects {
                        nes.ppu.latch = nes.ppu.read_byte(&mut *nes.mapper, ppu_addr);
                        if nes.ppu.rendering_enabled() && 
                        (nes.ppu.current_scanline == 261 ||
                         nes.ppu.current_scanline <= 239) {
                            // Glitchy increment, a fine y and a coarse x 
                            nes.ppu.increment_coarse_x();
                            nes.ppu.increment_fine_y();
                        } else {
                            // Normal incrementing behavior based on PPUCTRL
                            if nes.ppu.control & 0x04 == 0 {
                                nes.ppu.current_vram_address += 1;
                            } else {
                                nes.ppu.current_vram_address += 32;
                            }
                            nes.ppu.current_vram_address &= 0b0111_1111_1111_1111;
                        }
                        return nes.ppu.latch;
                    } else {
                        return nes.ppu.read_byte(&mut *nes.mapper, ppu_addr);
                    }
                },
                _ => return 0
            }
        },
        0x4015 => {
            return nes.apu.read_register(address);
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
        0x4020 ... 0xFFFF => return nes.mapper.read_byte(address),
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
                    // Shift the nametable select bits into the temporary vram address
                    //                                  yyy_nn_YYYYY_XXXXX
                    nes.ppu.temporary_vram_address &= 0b111_00_11111_11111;
                    nes.ppu.temporary_vram_address |= (data as u16) << 10;
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
                    nes.ppu.oam_addr = nes.ppu.oam_addr.wrapping_add(1);
                },
                // PPU SCROLL
                5 => {
                    if nes.ppu.write_toggle {
                        nes.ppu.scroll_y = data; // OLD

                        // Set coarse Y and fine y into temporary address
                        //                                  yyy_nn_YYYYY_XXXXX
                        nes.ppu.temporary_vram_address &= 0b000_11_00000_11111;
                        nes.ppu.temporary_vram_address |= ((data as u16) & 0b1111_1000) << 2;
                        nes.ppu.temporary_vram_address |= ((data as u16) & 0b111) << 12;

                        nes.ppu.write_toggle = false;
                    } else {
                        nes.ppu.scroll_x = data; // OLD

                        // Set coarse X into temporary address
                        //                                  yyy_nn_YYYYY_XXXXX
                        nes.ppu.temporary_vram_address &= 0b111_11_11111_00000;
                        nes.ppu.temporary_vram_address |= (data as u16) >> 3;
                        // Set fine X immediately
                        nes.ppu.fine_x = data & 0b111;

                        nes.ppu.write_toggle = true;
                    }
                },
                // PPU ADDR
                6 => {
                    if nes.ppu.write_toggle {
                        nes.ppu.temporary_vram_address &= 0b0111_1111_0000_0000;
                        nes.ppu.temporary_vram_address |= data as u16;
                        // Apply the final vram address immediately
                        nes.ppu.current_vram_address = nes.ppu.temporary_vram_address;

                        nes.ppu.write_toggle = false;
                    } else {
                        nes.ppu.temporary_vram_address &= 0b0000_0000_1111_1111;
                        // Note: This is missing bit 14 on purpose! This is cleared by the real PPU during
                        // the write to PPU ADDR for reasons unknown.
                        nes.ppu.temporary_vram_address |= ((data as u16) & 0b0011_1111) << 8;
                        nes.ppu.write_toggle = true;
                    }
                },
                // PPUDATA
                7 => {
                    let ppu_addr = nes.ppu.current_vram_address;
                    if nes.ppu.rendering_enabled() && 
                    (nes.ppu.current_scanline == 261 ||
                    nes.ppu.current_scanline <= 239) {
                        // Glitchy increment, a fine y and a coarse x 
                        nes.ppu.increment_coarse_x();
                        nes.ppu.increment_fine_y();
                    } else {
                        // Normal incrementing behavior based on PPUCTRL
                        if nes.ppu.control & 0x04 == 0 {
                            nes.ppu.current_vram_address += 1;
                        } else {
                            nes.ppu.current_vram_address += 32;
                        }
                        nes.ppu.current_vram_address &= 0b0111_1111_1111_1111;
                    }
                    nes.ppu.write_byte(&mut *nes.mapper, ppu_addr, data);
                },
                _ => ()
            }
        },
        0x4000 ... 0x4013 => {
            nes.apu.write_register(address, data);
        },
        0x4014 => {
            // OAM DMA, for cheating just do this instantly and return
            let read_address = (data as u16) << 8;
            for i in 0 .. 256 {
                let byte = read_byte(nes, read_address + i);
                nes.ppu.oam[i as usize] = byte;
            }
        },
        0x4015 => {
            nes.apu.write_register(address, data);
        },
        0x4016 => {
            // Input latch
            nes.input_latch = data & 0x1 != 0;
            if nes.input_latch {
                nes.p1_data = nes.p1_input;
                nes.p2_data = nes.p2_input;
            }
        },
        0x4017 => {
            nes.apu.write_register(address, data);
        },
        0x4020 ... 0xFFFF => nes.mapper.write_byte(address, data),
        _ => () // Do nothing!
    }
}
