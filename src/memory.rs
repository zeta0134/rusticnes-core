use std::ops::Index;
use std::ops::IndexMut;

pub struct PpuMemory {
    pub pattern_0: [u8; 0x1000],
    pub pattern_1: [u8; 0x1000],
    pub internal_vram: [u8; 0x800],
    pub oam: [u8; 0x100],
    pub palette: [u8; 0x20],

    pub v_mirroring: bool,
}

impl PpuMemory {
    pub fn read_byte(&mut self, address: u16) -> u8 {
        let masked_address = address & 0x3FFF;
        match masked_address {
            0x0000 ... 0x0FFF => return self.pattern_0[(masked_address & 0x1000) as usize],
            0x1000 ... 0x1FFF => return self.pattern_1[(masked_address & 0x1000) as usize],
            // Nametable 0
            0x2000 ... 0x23FF => return self.internal_vram[(masked_address & 0x3FF) as usize],
            // Nametable 1
            0x2400 ... 0x27FF => {
                if self.v_mirroring {
                    return self.internal_vram[((masked_address & 0x3FF) + 0x400) as usize];
                } else {
                    return self.internal_vram[(masked_address & 0x3FF) as usize];
                }
            },
            // Nametable 2
            0x2800 ... 0x2BFF => {
                if self.v_mirroring {
                    return self.internal_vram[(masked_address & 0x3FF) as usize];
                } else {
                    return self.internal_vram[((masked_address & 0x3FF) + 0x400) as usize];
                }
            },
            0x2C00 ... 0x2FFF => return self.internal_vram[((masked_address & 0x3FF) + 0x400) as usize],
            0x3000 ... 0x3EFF => return self.read_byte(masked_address - 0x1000),
            0x3F00 ... 0x3FFF => {
                let mut palette_address = masked_address & 0x1F;
                // Weird background masking
                if palette_address & 0x13 == 0x10 {
                    palette_address = palette_address - 0x10;
                }
                return self.palette[palette_address as usize];
            },
            _ => return 0
        }
    }
    pub fn write_byte(&mut self, address: u16, data: u8) {
        let masked_address = address & 0x3FFF;
        match masked_address {
            0x0000 ... 0x0FFF => self.pattern_0[(masked_address & 0x1000) as usize] = data,
            0x1000 ... 0x1FFF => self.pattern_1[(masked_address & 0x1000) as usize] = data,
            // Nametable 0
            0x2000 ... 0x23FF => self.internal_vram[(masked_address & 0x3FF) as usize] = data,
            // Nametable 1
            0x2400 ... 0x27FF => {
                if self.v_mirroring {
                    self.internal_vram[((masked_address & 0x3FF) + 0x400) as usize] = data;
                } else {
                    self.internal_vram[(masked_address & 0x3FF) as usize] = data;
                }
            },
            // Nametable 2
            0x2800 ... 0x2BFF => {
                if self.v_mirroring {
                    self.internal_vram[(masked_address & 0x3FF) as usize] = data;
                } else {
                    self.internal_vram[((masked_address & 0x3FF) + 0x400) as usize] = data;
                }
            },
            0x2C00 ... 0x2FFF => self.internal_vram[((masked_address & 0x3FF) + 0x400) as usize] = data,
            0x3000 ... 0x3EFF => self.write_byte(masked_address - 0x1000, data),
            0x3F00 ... 0x3FFF => {
                let mut palette_address = masked_address & 0x1F;
                // Weird background masking
                if palette_address & 0x13 == 0x10 {
                    palette_address = palette_address - 0x10;
                }
                self.palette[palette_address as usize] = data;
            },
            _ => () // Do nothing!
        }
    }
}

pub struct CpuMemory {
    // Naive implementation -- a stupid array!
    //pub raw: [u8; 0x10000]

    pub iram_raw: [u8; 0x800],

    // PPU Registers
    ppu_memory: PpuMemory,
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
            ppu_memory: PpuMemory {
                pattern_0: [0u8; 0x1000],
                pattern_1: [0u8; 0x1000],
                internal_vram: [0u8; 0x800],
                oam: [0u8; 0x100],
                palette: [0u8; 0x20],
                v_mirroring: false,
            }
        }
    }

    pub fn passively_read_byte(&mut self, address: u16) -> u8 {
        return self._read_byte(address, false);
    }

    pub fn read_byte(&mut self, address: u16) -> u8 {
        return self._read_byte(address, true);
    }

    fn _read_byte(&mut self, address: u16, side_effects: bool) -> u8 {
        match address {
            0x0000 ... 0x1FFF => return self.iram_raw[(address & 0x7FF) as usize],
            0x2000 ... 0x3FFF => {
                // PPU
                let ppu_reg = address & 0x7;
                println!("PPU Register Read: {}", ppu_reg);
                match ppu_reg {
                    // PPUCTRL, PPUMASK, OAMADDR | PPUSCROLL | PPUADDR (Write Only)
                    0 | 1 | 3 | 5 | 6 => {
                        return self.ppu_latch;
                    },
                    // PPUSTATUS
                    2 => {
                        if side_effects {
                            println!("Status Register Read: {}", self.ppu_status);
                            self.ppu_select_scroll_y = false;
                            self.ppu_select_low = false;
                            self.ppu_latch = (self.ppu_status & 0xE0) + (self.ppu_latch & 0x1F);
                            self.ppu_status = self.ppu_status & 0x7F; // Clear VBlank bit
                            return self.ppu_latch;
                        } else {
                            return self.ppu_status & 0xE0 + self.ppu_latch & 0x1F;
                        }
                    },
                    // OAMDATA
                    4 => {
                        if side_effects {
                            self.ppu_latch = self.ppu_memory.oam[self.oam_addr as usize];
                            return self.ppu_latch;
                        } else {
                            return self.ppu_memory.oam[self.oam_addr as usize];
                        }
                    },
                    // PPUDATA
                    7 => {
                        if side_effects {
                            self.ppu_latch = self.ppu_memory.read_byte(self.ppu_addr);
                            return self.ppu_latch;
                        } else {
                            return self.ppu_memory.read_byte(self.ppu_addr);
                        }
                    },
                    _ => return 0
                }
            },
            0x8000 ... 0xFFFF => return self.cart_rom[(address & 0x7FFF) as usize],
            _ => return 0
        }
    }

    pub fn write_byte(&mut self, address: u16, data: u8) {
        match address {
            0x0000 ... 0x1FFF => self.iram_raw[(address & 0x7FF)    as usize] = data,
            0x2000 ... 0x3FFF => {
                // PPU
                let ppu_reg = address & 0x7;
                self.ppu_latch = data;
                match ppu_reg {
                    // PPUCTRL
                    0 => {
                        self.ppu_control = data;
                    },
                    // PPU MASK
                    1 => {
                        self.ppu_mask = data;
                    },
                    // PPUSTATUS
                    2 => {
                        self.ppu_status = data & 0xE0;
                    },
                    // OAM ADDRESS
                    3 => {
                        self.oam_addr = data;
                    },
                    // OAMDATA
                    4 => {
                        self.ppu_memory.oam[self.oam_addr as usize] = data;
                    },
                    // PPU SCROLL
                    5 => {
                        if self.ppu_select_scroll_y {
                            self.ppu_scroll_y = data;
                            self.ppu_select_scroll_y = false;
                        } else {
                            self.ppu_scroll_x = data;
                            self.ppu_select_scroll_y = true;
                        }
                    },
                    // PPU ADDR
                    6 => {
                        if self.ppu_select_low {
                            self.ppu_addr = (self.ppu_addr & 0xFF00) + data as u16;
                            self.ppu_select_scroll_y = false;
                        } else {
                            self.ppu_addr = (self.ppu_addr & 0xFF) + ((data as u16) << 8);
                            self.ppu_select_low = true;
                        }
                    },
                    // PPUDATA
                    7 => {
                        self.ppu_memory.write_byte(self.ppu_addr, data);
                    },
                    _ => ()
                }
            }
            _ => () // Do nothing!
        }
    }
}
