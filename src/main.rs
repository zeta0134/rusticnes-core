extern crate find_folder;
extern crate image;
extern crate piston_window;

use piston_window::*;
use piston_window::Button::Keyboard;
use piston_window::Key;

mod apu;
mod cartridge;
mod cpu;
mod debug;
mod mmc;
mod nes;
mod memory;
mod palettes;
mod ppu;

use std::env;
use std::error::Error;
use std::io::Read;
use std::fs::File;

use image::ImageBuffer;
use image::Rgba;

use nes::NesState;
use palettes::NTSC_PAL;

fn main() {
    println!("Welcome to RusticNES");

    //let mut window: PistonWindow = WindowSettings::new("RusticNES", [1024, 768])
    //.exit_on_esc(true).opengl(OpenGL::V3_1).build().unwrap();
    //window.set_ups(60);

    let mut window: PistonWindow = PistonWindow::new(OpenGL::V4_0, 0,
        WindowSettings::new("RusticNES", (1024, 768))
            .srgb(false)
            .build()
            .unwrap());
    window.set_ups(60);

    let args: Vec<String> = env::args().collect();
    let filename = &args[1];

    println!("Attempting to load {}...", filename);

    let mut file = match File::open(filename) {
        Err(why) => panic!("Couldn't open mario.nes: {}", why.description()),
        Ok(file) => file,
    };
    let mut cartridge = Vec::new();
    // Read the whole damn thing?
    match file.read_to_end(&mut cartridge) {
        Err(why) => panic!("Couldn't read data: {}", why.description()),
        Ok(bytes_read) => println!("Data read successfully: {}", bytes_read),
    };

    let nes_header = cartridge::extract_header(&cartridge);
    cartridge::print_header_info(nes_header);
    let mapper = cartridge::load_from_cartridge(nes_header, &cartridge);
    let mut nes = NesState::new(mapper);

    // Initialize CPU register state for power-up sequence
    nes.registers.a = 0;
    nes.registers.y = 0;
    nes.registers.x = 0;
    nes.registers.s = 0xFD;

    let pc_low = memory::read_byte(&mut nes, 0xFFFC);
    let pc_high = memory::read_byte(&mut nes, 0xFFFD);
    nes.registers.pc = pc_low as u16 + ((pc_high as u16) << 8);

    // "Screen"
    let texture_settings = TextureSettings::new()
        .min(texture::Filter::Nearest)
        .mag(texture::Filter::Nearest);

    // Load a font for text drawing (todo: probably need to find one with a better license)
    /*
    let assets = find_folder::Search::ParentsThenKids(3, 3)
        .for_folder("assets").unwrap();
    let ref font = assets.join("FiraSans-Regular.ttf");
    let factory = window.factory.clone();
    let mut glyphs = Glyphs::new(font, factory,
        TextureSettings::new()
        .min(texture::Filter::Nearest)
        .mag(texture::Filter::Nearest)).unwrap();
    // */

    let mut screen_buffer = ImageBuffer::new(256, 240);
    let mut screen_texture = Texture::from_image(
        &mut window.factory,
        &screen_buffer,
        &texture_settings
    ).unwrap();

    let mut pattern_0_buffer = ImageBuffer::new(128, 128);
    let mut pattern_1_buffer = ImageBuffer::new(128, 128);
    debug::generate_chr_pattern(&mut *nes.mapper, 0x0000, &mut pattern_0_buffer);
    debug::generate_chr_pattern(&mut *nes.mapper, 0x1000, &mut pattern_1_buffer);
    let mut pattern_0_texture = Texture::from_image(&mut window.factory, &pattern_0_buffer,
        &texture_settings).unwrap();
    let mut pattern_1_texture = Texture::from_image(&mut window.factory, &pattern_1_buffer,
        &texture_settings).unwrap();

    let mut nametables_buffer = ImageBuffer::new(512, 480);
    let mut nametables_texture = Texture::from_image(&mut window.factory, &nametables_buffer,
        &texture_settings).unwrap();

    let mut audiocanvas_buffer = ImageBuffer::new(256, 128);
    let mut audiocanvas_texture = Texture::from_image(&mut window.factory, &audiocanvas_buffer,
        &texture_settings).unwrap();

    let mut thingy = 0;
    let mut running = false;
    let mut memory_viewer_page = 0u16;

    let key_mappings: [Key; 8] = [
        Key::X,
        Key::Z,
        Key::RShift,
        Key::Return,
        Key::Up,
        Key::Down,
        Key::Left,
        Key::Right,
    ];

    debug::print_program_state(&mut nes);

    while let Some(event) = window.next() {
        if let Some(button) = event.press_args() {
            // NES Key State
            for i in 0 .. 8 {
                if button == Keyboard(key_mappings[i]) {
                    // Set the corresponding bit
                    nes.p1_input |= 0x1 << i;
                }
            }
        }

        if let Some(button) = event.release_args() {
            // NES Key State
            for i in 0 .. 8 {
                if button == Keyboard(key_mappings[i]) {
                    // Clear the corresponding bit
                    nes.p1_input &= (0x1 << i) ^ 0xFF;
                }
            }

            // Keyboard input here
            if button == Keyboard(Key::R) {
                running = !running;
            }

            if button == Keyboard(Key::Space) {
                // Run one opcode, then debug
                nes::step(&mut nes);
                debug::print_program_state(&mut nes);
            }

            if button == Keyboard(Key::H) {
                // Run one opcode, then debug
                nes::run_until_hblank(&mut nes);
                debug::print_program_state(&mut nes);
            }

            if button == Keyboard(Key::V) {
                // Run one opcode, then debug
                nes::run_until_vblank(&mut nes);
                debug::print_program_state(&mut nes);
            }

            if button == Keyboard(Key::Comma ) {
                memory_viewer_page = memory_viewer_page.wrapping_sub(0x100);
                if memory_viewer_page == 0x1F00 {
                    memory_viewer_page = 0x0700;
                }
                if memory_viewer_page == 0x3F00 {
                    memory_viewer_page = 0x2000;
                }
            }
            if button == Keyboard(Key::Period) {
                memory_viewer_page = memory_viewer_page.wrapping_add(0x100);
                if memory_viewer_page == 0x0800 {
                    memory_viewer_page = 0x2000;
                }
                if memory_viewer_page == 0x2100 {
                    memory_viewer_page = 0x4000;
                }
            }
        }

        if let Some(_) = event.update_args() {
            // Debug draw some junk
            for x in 0 .. 256 {
                for y in 0 .. 240 {
                    let palette_index = ((nes.ppu.screen[y * 256 + x]) as usize) * 3;
                    screen_buffer.put_pixel(x as u32, y as u32, Rgba { data: [
                        NTSC_PAL[palette_index + 0],
                        NTSC_PAL[palette_index + 1],
                        NTSC_PAL[palette_index + 2],
                        255] });
                }
            }
            let _ = screen_texture.update(&mut window.encoder, &screen_buffer);
            debug::generate_nametables(&mut *nes.mapper, &mut nes.ppu, &mut nametables_buffer);
            debug::generate_chr_pattern(&mut *nes.mapper, 0x0000, &mut pattern_0_buffer);
            debug::generate_chr_pattern(&mut *nes.mapper, 0x1000, &mut pattern_1_buffer);
            let _ = nametables_texture.update(&mut window.encoder, &nametables_buffer);
            let _ = pattern_0_texture.update(&mut window.encoder, &pattern_0_buffer);
            let _ = pattern_1_texture.update(&mut window.encoder, &pattern_1_buffer);

            if running {
                nes::run_until_vblank(&mut nes);
            }

            // Draw audio samples! What could possibly go wrong?
            // Why do we need to clear this manually?
            //*

            // Background
            for x in 0 .. 256 {
                for y in   0 ..  25 { audiocanvas_buffer.put_pixel(x, y, Rgba { data: [32,  8,  8, 255] }); }
                for y in  25 ..  50 { audiocanvas_buffer.put_pixel(x, y, Rgba { data: [32, 16,  8, 255] }); }
                for y in  50 ..  75 { audiocanvas_buffer.put_pixel(x, y, Rgba { data: [ 8, 32,  8, 255] }); }
                for y in  75 .. 100 { audiocanvas_buffer.put_pixel(x, y, Rgba { data: [ 8, 16, 32, 255] }); }
                for y in 100 .. 125 { audiocanvas_buffer.put_pixel(x, y, Rgba { data: [16, 16, 16, 255] }); }
            }

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
            draw_waveform(&mut audiocanvas_buffer, &nes.apu.pulse_1.debug_buffer,  nes.apu.buffer_index, Rgba { data: [192,  32,  32, 255]}, 0,   0, 256,  25, 16);
            draw_waveform(&mut audiocanvas_buffer, &nes.apu.pulse_2.debug_buffer,  nes.apu.buffer_index, Rgba { data: [192,  96,  32, 255]}, 0,  25, 256,  25, 16);
            draw_waveform(&mut audiocanvas_buffer, &nes.apu.triangle.debug_buffer, nes.apu.buffer_index, Rgba { data: [ 32, 192,  32, 255]}, 0,  50, 256,  25, 16);
            draw_waveform(&mut audiocanvas_buffer, &nes.apu.noise.debug_buffer,    nes.apu.buffer_index, Rgba { data: [ 32,  96, 192, 255]}, 0,  75, 256,  25, 16);
            draw_waveform(&mut audiocanvas_buffer, &nes.apu.sample_buffer,         nes.apu.buffer_index, Rgba { data: [192, 192, 192, 255]}, 0, 100, 256,  25, 32768);

            let _ = audiocanvas_texture.update(&mut window.encoder, &audiocanvas_buffer);
            // */

        }

        window.draw_2d(&event, |context, graphics| {
            clear([0.8; 4], graphics);
            let base_transform = context.transform.scale(2.0, 2.0);
            image(&screen_texture, base_transform, graphics);

            let pattern_0_transform = base_transform.trans(256.0, 0.0);
            let pattern_1_transform = base_transform.trans(256.0 + 128.0, 0.0);
            image(&pattern_0_texture, pattern_0_transform, graphics);
            image(&pattern_1_texture, pattern_1_transform, graphics);

            let nametables_transform = context.transform.trans(512.0, 256.0);
            image(&nametables_texture, nametables_transform, graphics);

            let audiocanvas_transform = base_transform.trans(0.0, 240.0);
            image(&audiocanvas_texture, audiocanvas_transform, graphics);

            /*
            let base_text_transform = context.transform.trans(0.0, 0.0 + 16.0);
            let black_text = text::Text::new_color([0.0, 0.0, 0.0, 1.0], 16);
            let bright_text = text::Text::new_color([1.0, 1.0, 1.0, 0.8], 16);
            let dim_text = text::Text::new_color([1.0, 1.0, 1.0, 0.3], 16);
            let memory_viewer_base = base_text_transform.trans(0.0, 480.0);
            black_text.draw("--- MEMORY ---", &mut glyphs, &context.draw_state, memory_viewer_base, graphics);

            for y in 0 .. 16 {
                black_text.draw(&format!("0x{:04X}:", memory_viewer_page + y * 16),
                    &mut glyphs, &context.draw_state, memory_viewer_base.trans(0.0, y as f64 * 17.0 + 16.0), graphics);
                for x in 0 .. 16 {
                    let mut color = [0.15, 0.15, 0.15, 1.0];
                    if (x ^ y) & 0x1 != 0 {
                        color = [0.25, 0.25, 0.25, 1.0];
                    }
                    let address = (y * 16 + x) as u16 + memory_viewer_page;
                    if address == nes.registers.pc {
                        color = [0.5, 0.1, 0.1, 1.0];
                    } else if address == (nes.registers.s as u16 + 0x100) {
                        color = [0.1, 0.1, 0.5, 1.0];
                    } else if nes.memory.recent_reads.contains(&address) {
                        for i in 0 .. nes.memory.recent_reads.len() {
                            if nes.memory.recent_reads[i] == address {
                                let brightness = 0.6 - (0.02 * i as f32);
                                color = [0.3, brightness, 0.3, 1.0];
                                break;
                            }
                        }
                    } else if nes.memory.recent_writes.contains(&address) {
                        for i in 0 .. nes.memory.recent_writes.len() {
                            if nes.memory.recent_writes[i] == address {
                                let brightness = 0.6 - (0.02 * i as f32);
                                color = [brightness, brightness, 0.2, 1.0];
                                break;
                            }
                        }
                    }
                    let byte = memory::passively_read_byte(&mut nes, address);
                    let tx = x as f64 * 22.0 + 80.0;
                    let ty = y as f64 * 17.0 + 16.0;
                    let pos = memory_viewer_base.trans(tx, ty);
                    rectangle(color, [0.0, 0.0, 22.0, 17.0], pos.trans(-2.0, -14.0), graphics);

                    if byte == 0 {
                        dim_text.draw(&format!("{:02X}", byte),
                            &mut glyphs, &context.draw_state, pos, graphics);
                    } else {
                        bright_text.draw(&format!("{:02X}", byte),
                            &mut glyphs, &context.draw_state, pos, graphics);
                    }
                }
            }
            // */

            thingy = thingy + 1;
        });
    }
}
