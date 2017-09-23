use image::ImageBuffer;
use image::Rgba;

use apu::ApuState;
use memory;
use mmc::mapper::*;
use nes::NesState;
use palettes::NTSC_PAL;
use ppu;



fn draw_waveform(imagebuffer: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, audiobuffer: &[u16], start_index: usize, color: Rgba<u8>, x: u32, y: u32, width: u32, height: u32, scale: u32) {
    let mut last_y = 0;
    for dx in x .. (x + width) {
        let sample_index = (start_index + dx as usize) % audiobuffer.len();
        let sample = audiobuffer[sample_index];
        let current_x = dx as u32;
        let mut current_y = ((sample as u32 * height) / scale) as u32;
        if current_y >= height {
            current_y = height - 1;
        }
        for dy in current_y .. last_y {
            imagebuffer.put_pixel(current_x, y + dy, color);
        }
        for dy in last_y .. current_y {
            imagebuffer.put_pixel(current_x, y + dy, color);
        }
        last_y = current_y;
        imagebuffer.put_pixel(dx, y + current_y, color);
    }
}

pub fn draw_audio_samples(apu: &ApuState, mut audiocanvas_buffer: &mut ImageBuffer<Rgba<u8>, Vec<u8>>) {
    // Draw audio samples! What could possibly go wrong?
    // Do we need to clear this manually?
    //*

    // Background
    for x in 0 .. 256 {
        for y in   0 ..  150 { audiocanvas_buffer.put_pixel(x, y, Rgba { data: [8,  8,  8, 255] }); }
        if !(apu.pulse_1.debug_disable) {
            for y in   0 ..  25 { audiocanvas_buffer.put_pixel(x, y, Rgba { data: [32,  8,  8, 255] }); }
        }
        if !(apu.pulse_2.debug_disable) {
            for y in  25 ..  50 { audiocanvas_buffer.put_pixel(x, y, Rgba { data: [32, 16,  8, 255] }); }
        }
        if !(apu.triangle.debug_disable) {
            for y in  50 ..  75 { audiocanvas_buffer.put_pixel(x, y, Rgba { data: [ 8, 32,  8, 255] }); }
        }
        if !(apu.noise.debug_disable) {
            for y in  75 .. 100 { audiocanvas_buffer.put_pixel(x, y, Rgba { data: [ 8, 16, 32, 255] }); }
        }
        if !(apu.dmc.debug_disable) {
            for y in  100 .. 125 { audiocanvas_buffer.put_pixel(x, y, Rgba { data: [ 16, 8, 32, 255] }); }
        }
        for y in 125 .. 150 { audiocanvas_buffer.put_pixel(x, y, Rgba { data: [16, 16, 16, 255] }); }
    }

    if !(apu.pulse_1.debug_disable) {
        draw_waveform(&mut audiocanvas_buffer, &apu.pulse_1.debug_buffer,
            apu.buffer_index, Rgba { data: [192,  32,  32, 255]}, 0,   0, 256,  25, 16);
    }
    if !(apu.pulse_2.debug_disable) {
        draw_waveform(&mut audiocanvas_buffer, &apu.pulse_2.debug_buffer,
            apu.buffer_index, Rgba { data: [192,  96,  32, 255]}, 0,  25, 256,  25, 16);
    }
    if !(apu.triangle.debug_disable) {
        draw_waveform(&mut audiocanvas_buffer, &apu.triangle.debug_buffer,
            apu.buffer_index, Rgba { data: [ 32, 192,  32, 255]}, 0,  50, 256,  25, 16);
    }
    if !(apu.noise.debug_disable) {
        draw_waveform(&mut audiocanvas_buffer, &apu.noise.debug_buffer,
            apu.buffer_index, Rgba { data: [ 32,  96, 192, 255]}, 0,  75, 256,  25, 16);
    }
    if !(apu.dmc.debug_disable) {
        draw_waveform(&mut audiocanvas_buffer, &apu.dmc.debug_buffer,
            apu.buffer_index, Rgba { data: [ 96,  32, 192, 255]}, 0, 100, 256,  25, 128);
    }
    draw_waveform(&mut audiocanvas_buffer, &apu.sample_buffer,
        apu.buffer_index, Rgba { data: [192, 192, 192, 255]}, 0, 125, 256,  25, 16384);
}

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
    println!("\nMemory @ Program Counter");
    // print out the next 8 bytes or so from the program counter
    let mut pc = registers.pc;
    for _ in 1 .. 8 {
        println!("0x{:04X}: 0x{:02X}", pc, memory::passively_read_byte(nes, pc));
        pc = pc.wrapping_add(1);
    }

    let mirror_mode = match nes.mapper.mirroring() {
        Mirroring::Horizontal => "Horizontal",
        Mirroring::Vertical => "Vertical",
        Mirroring::OneScreenLower => "OneScreen - Lower",
        Mirroring::OneScreenUpper => "OneScreen - Upper",
        Mirroring::FourScreen => "FourScreen",
    };

    println!("\nPPU: Control: {:02X} Mask: {:02X} Status: {:02X}, Scroll: {:02X}, {:02X}",
        nes.ppu.control, nes.ppu.mask, nes.ppu.status, nes.ppu.scroll_x, nes.ppu.scroll_y);
    println!("OAM Address: {:04X} PPU Address: {:04X}",
        nes.ppu.oam_addr, nes.ppu.current_addr);
    println!("Frame: {}, Scanline: {}, M. Clock: {}, Scanline Cycles: {}, Mirroring: {}\n",
        nes.ppu.current_frame, nes.ppu.current_scanline, nes.master_clock, nes.ppu.scanline_cycles, mirror_mode);
    nes.mapper.print_debug_status();
}
