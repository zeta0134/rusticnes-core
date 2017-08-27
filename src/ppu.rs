// Note: For basic testing purposes, this is scanline-accurate. This should
// later be rewritten with cycle-accurate logic once we're past proof of concept
// and prototype stages.

pub struct PpuState {
    // PPU Memory (incl. cart CHR ROM for now)
    pub pattern_0: [u8; 0x1000],
    pub pattern_1: [u8; 0x1000],
    pub internal_vram: [u8; 0x800],
    pub oam: [u8; 0x100],
    pub palette: [u8; 0x20],

    pub v_mirroring: bool,

    // Memory Mapped Registers
    // PPU Registers
    pub latch: u8,

    pub read_buffer: u8,

    pub control: u8,
    pub mask: u8,
    pub status: u8,
    pub oam_addr: u8,

    // Scrolling, which is implemented with a flip/flop register
    pub select_scroll_y: bool,
    pub scroll_x: u8,
    pub scroll_y: u8,

    // PPU Address, similar to scrolling, has a high / low component
    pub select_low: bool,
    pub current_addr: u16,

    pub oam_dma_high: u8,

    // Internal
    pub current_frame: u32,
    pub current_scanline: u16,
    pub scanline_cycles: u32,
    pub last_cycle: u32,

    // Framebuffer
    pub screen: [u8; 256 * 240],
    pub sprite_color: [u8; 256],
    pub sprite_index: [u8; 256],
    pub sprite_bg_priority: [bool; 256],
    pub sprite_zero: [bool; 256],
}

impl PpuState {
    pub fn new() -> PpuState {
        return PpuState {
           pattern_0: [0u8; 0x1000],
           pattern_1: [0u8; 0x1000],
           internal_vram: [0u8; 0x800],
           oam: [0u8; 0x100],
           palette: [0u8; 0x20],
           v_mirroring: true,
           current_frame: 0,
           current_scanline: 0,
           scanline_cycles: 0,
           last_cycle: 0,
           screen: [0u8; 256 * 240],
           sprite_color: [0u8; 256],
           sprite_index: [0u8; 256],
           sprite_bg_priority: [false; 256],
           sprite_zero: [false; 256],

           control: 0,
           mask: 0,
           status: 0,
           oam_addr: 0,
           select_scroll_y: false,
           scroll_x: 0,
           scroll_y: 0,
           select_low: false,
           current_addr: 0,
           oam_dma_high: 0,
           latch: 0,
           read_buffer: 0,
       };
    }

    pub fn read_byte(&mut self, address: u16) -> u8 {
        let masked_address = address & 0x3FFF;
        match masked_address {
            0x3F00 ... 0x3FFF => {
                // Weird read buffer behavior
                self.read_buffer = self.read_byte((masked_address & 0x0FFF) + 0x2000);
                return self._read_byte(address);
            },
            _ => {
                let result = self.read_buffer;
                self.read_buffer = self._read_byte(address);
                return result;
            }
        }
    }

    pub fn _read_byte(&mut self, address: u16) -> u8 {
        let masked_address = address & 0x3FFF;
        match masked_address {
            0x0000 ... 0x0FFF => return self.pattern_0[(masked_address & 0x0FFF) as usize],
            0x1000 ... 0x1FFF => return self.pattern_1[(masked_address & 0x0FFF) as usize],
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
            // Nametable 3
            0x2C00 ... 0x2FFF => return self.internal_vram[((masked_address & 0x3FF) + 0x400) as usize],
            0x3000 ... 0x3EFF => return self.read_byte(masked_address - 0x1000),
            0x3F00 ... 0x3FFF => {
                // Weird read buffer behavior
                //self.read_buffer = self.read_byte((masked_address & 0x0FFF) + 0x2000);

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
            //0x0000 ... 0x0FFF => self.pattern_0[(masked_address & 0x0FFF) as usize] = data,
            //0x1000 ... 0x1FFF => self.pattern_1[(masked_address & 0x0FFF) as usize] = data,
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

    fn render_sprites(&mut self, scanline: u8) {
        // Init buffers
        self.sprite_color = [0u8; 256];
        self.sprite_index = [0u8; 256];
        self.sprite_bg_priority = [false; 256];
        self.sprite_zero = [false; 256];

        let mut secondary_oam = [0xFFu8; 32];
        let mut secondary_index = 0;
        let sprite_size = 8;

        // Gather first 8 visible sprites (and pay attention if there are more)
        for i in 0 .. 40 {
            let y = self.oam[i * 4 + 0];
            if scanline >= y && scanline < y + sprite_size {
                if secondary_index < 8 {
                    for j in 0 .. 4 {
                        secondary_oam[secondary_index * 4 + j] = self.oam[i * 4 + j];
                    }
                    secondary_index += 1;
                } else {
                    self.status = self.status | 0x20; // bit 5 = sprite overflow this frame
                }
            }
        }

        // secondary_oam now has up to 8 sprites, all of which have a Y coordinate
        // which is on this scanline. Proceed to render!
        let pattern = self.pattern_0;

        // Note: Iterating over the list in reverse order cheats a bit, by having higher priority
        // sprites overwrite the work done to draw lower priority sprites.
        for i in (0 .. secondary_index).rev() {
            let sprite_y = secondary_oam[i * 4 + 0];
            let tile_index = secondary_oam[i * 4 + 1];
            let flags = secondary_oam[i * 4 + 2];
            let sprite_x = secondary_oam[i * 4 + 3];

            let priority = flags & 0x20 != 0;
            let mut tile_y = scanline - sprite_y;
            if flags & 0x80 != 0 {
                tile_y = 7 - tile_y;
            }

            for x in 0 .. 8 {
                let scanline_x = (sprite_x as u16) + x;
                let mut tile_x = x;
                if flags & 0x40 != 0 {
                    tile_x = 7 - tile_x;
                }

                if scanline_x < 256 {
                    let palette_index = decode_chr_pixel(&pattern, tile_index, tile_x as u8, tile_y);
                    if palette_index  > 0 {
                        self.sprite_index[scanline_x as usize] = palette_index;
                        self.sprite_color[scanline_x as usize] = palette_index; // TODO: Decode real color here
                        self.sprite_bg_priority[scanline_x as usize] = priority;
                        self.sprite_zero[scanline_x as usize] = i == 0;
                    }
                }
            }
        }
    }

    fn render_background(&mut self, scanline: u16) {
        let mut pattern = self.pattern_0;
        if (self.control & 0x10) != 0 {
            pattern = self.pattern_1;
        }
        let mut y = self.scroll_y as u16 + scanline;
        if self.control & 0x2 != 0 {
            y += 240;
        }
        // wrap around if we go off the bottom of the map
        y = y % (240 * 2);
        for sx in 0 .. 256 {
            let mut x = sx + self.scroll_x as u16;
            if self.control & 0x1 != 0 {
                x += 256;
            }
            // wrap around if we go off the right of the map
            x = x & 0x1FF;
            let tx = (x >> 3) as u8;
            let ty = (y >> 3) as u8;
            let tile = self.get_bg_tile(tx, ty);
            let bg_index = decode_chr_pixel(&pattern, tile, (x & 0x7) as u8, (y & 0x7) as u8);
            let mut palette_index = self.get_bg_palette(tx, ty);
            if bg_index == 0 {
                palette_index = 0; // Ignore palette index for color 0
            }
            let palette_color = self._read_byte(((palette_index << 2) + bg_index) as u16 + 0x3F00);
            self.screen[(scanline * 256 + sx) as usize] = palette_color;

            // Here, decide if a sprite pixel should overwrite a background pixel
            if self.sprite_index[sx as usize] != 0 {
                if self.sprite_zero[sx as usize] {
                    self.status = self.status | 0x40; // bit 6 = sprite zero hit
                }
                if bg_index == 0 || !self.sprite_bg_priority[sx as usize] {
                    self.screen[(scanline * 256 + sx) as usize] = self.sprite_color[sx as usize];
                }
            }
        }
    }

    // Return value: NMI interrupt happened this cycle
    pub fn process_scanline(&mut self) {
        let scanline = self.current_scanline;
        match scanline {
            0 ... 239 => {
                // Visible scanline here
                self.render_sprites(scanline as u8);
                self.render_background(scanline);
            },
            // 240 does nothing
            241 => {
                // VBlank! Set NMI flag here
                self.status = (self.status & 0x7F) + 0x80; // Set VBlank bit
            },
            // 242 - 260 do nothing
            261 => {
                // Set vertical scrolling registers, in preparation for new frame
                // (Emulator: Draw actual frame here)
                // Clear sprite overflow and sprite zero hit too
                self.status = self.status & 0x1F; // Clear VBlank bit
            },
            _ => ()
        }
    }

    pub fn run_to_cycle(&mut self, current_cycle: u32) {
        let cycles_per_scanline = 341 * 4;
        self.scanline_cycles = self.scanline_cycles + (current_cycle - self.last_cycle);
        self.last_cycle = current_cycle;
        while self.scanline_cycles > cycles_per_scanline {
            self.process_scanline();
            self.current_scanline = self.current_scanline.wrapping_add(1);
            if self.current_scanline > 261 {
                self.current_scanline = 0;
                self.current_frame = self.current_frame + 1;
            }
            self.scanline_cycles = self.scanline_cycles  - cycles_per_scanline;
        }
    }

    pub fn get_bg_tile(&mut self, tx: u8, ty: u8) -> u8 {
        let mut address: u16 = 0x2000;
        if tx > 31 {
            address = address + 0x0400;
        }
        if ty > 29 {
            address = address + 0x0800;
        }
        address = address + ((ty % 30) as u16) * 32 + ((tx & 0x1F) as u16);
        return self._read_byte(address);
    }

    pub fn get_bg_palette(&mut self, tx: u8, ty: u8) -> u8 {
        let mut address: u16 = 0x23C0;
        if tx > 31 {
            address = address + 0x0400;
        }
        if ty > 29 {
            address = address + 0x0800;
        }
        address += ((tx & 0x1F) >> 2) as u16;
        address += (((ty % 30) >> 2) * 0x8) as u16;
        let attr_byte = self._read_byte(address);
        let shift = (((tx & 0x2) >> 1) + (ty & 0x2)) << 1;
        let mask = 0x3 << shift;
        return (attr_byte & mask) >> shift;
    }
}

// Given a pattern and tile / pixel coordinates, decodes the palette index and returns it
// (Palette index will be between 0 .. 3)
pub fn decode_chr_pixel(pattern: &[u8], tile: u8, pixel_x: u8, pixel_y: u8) -> u8 {
    let low_addr = (tile as u16) * 16 + (pixel_y as u16);
    let high_addr = low_addr + 8;
    let low_bit = pattern[low_addr as usize] >> (7 - pixel_x) & 0x1;
    let high_bit = pattern[high_addr as usize] >> (7 - pixel_x) & 0x1;
    return (high_bit << 1) + low_bit;
}
