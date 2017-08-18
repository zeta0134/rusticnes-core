use nes::NesState;
use memory::CpuMemory;

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
    pub current_scanline: u16,
    pub scanline_cycles: u32,
    pub last_cycle: u32,

    // Framebuffer
    pub screen: [u8; 256 * 240],
}

impl PpuState {
    pub fn new() -> PpuState {
        return PpuState {
           pattern_0: [0u8; 0x1000],
           pattern_1: [0u8; 0x1000],
           internal_vram: [0u8; 0x800],
           oam: [0u8; 0x100],
           palette: [0u8; 0x20],
           v_mirroring: false,
           current_scanline: 0,
           scanline_cycles: 0,
           last_cycle: 0,
           screen: [0u8; 256 * 240],

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
       };
    }

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

    // Return value: NMI interrupt happened this cycle
    pub fn process_scanline(&mut self) {
        let scanline = self.current_scanline;
        match scanline {
            0 ... 239 => {
                // Visible scanline here
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
                self.status = self.status & 0x7F; // Clear VBlank bit
            },
            _ => ()
        }
    }

    pub fn run_to_cycle(&mut self, cycles: u32, memory: &mut CpuMemory) {
        let cycles_per_scanline = 341 * 4;
        self.scanline_cycles = cycles - self.last_cycle;
        self.last_cycle = cycles;
        let nmi = false;
        while self.scanline_cycles > cycles_per_scanline {
            self.current_scanline.wrapping_add(1);
            self.scanline_cycles = self.scanline_cycles  - cycles_per_scanline;
            self.process_scanline();
        }
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
