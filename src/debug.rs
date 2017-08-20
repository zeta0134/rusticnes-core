use image::ImageBuffer;
use image::Rgba;

use nes::NesState;
use memory;
use ppu;

pub fn generate_chr_pattern(pattern: &[u8], buffer: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
    let debug_pallete: [u8; 4] = [255, 192, 128, 0];
    for x in 0 .. 16 {
        for y in 0 .. 16 {
            let tile = y * 16 + x;
            for px in 0 .. 8 {
                for py in 0 .. 8 {
                    let palette_index = ppu::decode_chr_pixel(pattern, tile as u8, px as u8, py as u8);
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

pub fn generate_nametables(ppu: &mut ppu::PpuState, buffer: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
    let mut pattern = ppu.pattern_0;
    if (ppu.control & 0x08) != 0 {
        pattern = ppu.pattern_1;
    }
    let debug_pallete: [u8; 4] = [255, 192, 128, 0];
    for tx in 0 .. 63 {
        for ty in 0 .. 59 {
            let tile_index = ppu.get_bg_tile(tx, ty);
            for px in 0 .. 8 {
                for py in 0 .. 8 {
                    let palette_index = ppu::decode_chr_pixel(&pattern, tile_index as u8, px as u8, py as u8);
                    buffer.put_pixel(tx as u32 * 8 + px as u32, ty as u32 * 8 + py as u32, Rgba { data: [
                        debug_pallete[palette_index as usize],
                        debug_pallete[palette_index as usize],
                        debug_pallete[palette_index as usize],
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
    println!("\nMemory @ Program Counter");
    // print out the next 8 bytes or so from the program counter
    let mut pc = registers.pc;
    for _ in 1 .. 8 {
        println!("0x{:04X}: 0x{:02X}", pc, memory::passively_read_byte(nes, pc));
        pc = pc.wrapping_add(1);
    }
    println!("\nPPU: Control: {:02X} Mask: {:02X} Status: {:02X}, Scroll: {:02X}, {:02X}",
        nes.ppu.control, nes.ppu.mask, nes.ppu.status, nes.ppu.scroll_x, nes.ppu.scroll_y);
    println!("OAM Address: {:04X} PPU Address: {:04X}",
        nes.ppu.oam_addr, nes.ppu.current_addr);
    println!("Frame: {}, Scanline: {}, Cycle: {}, Scanline Cycles: {}",
        nes.ppu.current_frame, nes.ppu.current_scanline, nes.current_cycle, nes.ppu.scanline_cycles);

}
