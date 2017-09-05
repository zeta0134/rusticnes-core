use image::ImageBuffer;
use image::Rgba;

use memory;
use mmc::mapper::Mapper;
use nes::NesState;
use palettes::NTSC_PAL;
use ppu;

pub fn generate_chr_pattern(mapper: &mut Mapper, pattern_address: u16, buffer: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
    let debug_pallete: [u8; 4] = [255, 192, 128, 0];
    for x in 0 .. 16 {
        for y in 0 .. 16 {
            let tile = y * 16 + x;
            for px in 0 .. 8 {
                for py in 0 .. 8 {
                    let palette_index = ppu::decode_chr_pixel(mapper, pattern_address, tile as u8, px as u8, py as u8);
                    buffer.put_pixel(x * 8 + px, y * 8 + py, Rgba { data: [
                        debug_pallete[palette_index as usize],
                        debug_pallete[palette_index as usize],
                        debug_pallete[palette_index as usize],
                        255] });
                }
            }
        }
    }
}

pub fn generate_nametables(mapper: &mut Mapper, ppu: &mut ppu::PpuState, buffer: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
    let mut pattern_address = 0x0000;
    if (ppu.control & 0x10) != 0 {
        pattern_address = 0x1000;
    }
    for tx in 0 .. 63 {
        for ty in 0 .. 59 {
            let tile_index = ppu.get_bg_tile(mapper, tx, ty);
            let palette_index = ppu.get_bg_palette(mapper, tx, ty);
            for px in 0 .. 8 {
                for py in 0 .. 8 {
                    let bg_index = ppu::decode_chr_pixel(mapper, pattern_address, tile_index as u8, px as u8, py as u8);
                    let mut palette_color = ppu._read_byte(mapper, ((palette_index << 2) + bg_index) as u16 + 0x3F00) as usize * 3;
                    if bg_index == 0 {
                        palette_color = ppu._read_byte(mapper, bg_index as u16 + 0x3F00) as usize * 3;
                    }
                    buffer.put_pixel(tx as u32 * 8 + px as u32, ty as u32 * 8 + py as u32, Rgba { data: [
                        NTSC_PAL[palette_color + 0],
                        NTSC_PAL[palette_color + 1],
                        NTSC_PAL[palette_color + 2],
                        255] });
                }
            }
        }
    }
}

pub fn print_program_state(nes: &mut NesState) {
    let registers = nes.registers;
    println!("=== NES State ===");
    println!("A: 0x{:02X} X: 0x{:02X} Y: 0x{:02X}", registers.a, registers.x, registers.y);
    println!("PC: 0x{:02X} S: 0x{:02X}", registers.pc, registers.s);
    println!("Flags: nv  dzic");
    println!("       {:b}{:b}  {:b}{:b}{:b}{:b}",
        registers.flags.negative as u8,
        registers.flags.overflow as u8,
        registers.flags.decimal as u8,
        registers.flags.zero as u8,
        registers.flags.interrupts_disabled as u8,
        registers.flags.carry as u8,
    );
    println!("\nPPU: Control: {:02X} Mask: {:02X} Status: {:02X}, Scroll: {:02X}, {:02X}",
        nes.ppu.control, nes.ppu.mask, nes.ppu.status, nes.ppu.scroll_x, nes.ppu.scroll_y);
    println!("OAM Address: {:04X} PPU Address: {:04X}",
        nes.ppu.oam_addr, nes.ppu.current_addr);
    println!("Frame: {}, Scanline: {}, Cycle: {}, Scanline Cycles: {}\n",
        nes.ppu.current_frame, nes.ppu.current_scanline, nes.current_cycle, nes.ppu.scanline_cycles);

    nes.mapper.print_debug_status();
}
