extern crate image;
extern crate piston_window;
extern crate glutin_window;

use piston_window::*;
use piston_window::Button::Keyboard;
use piston_window::Key;
use glutin_window::GlutinWindow;

mod cartridge;
mod cpu;
mod debug;
mod nes;
mod memory;
mod palettes;
mod ppu;

use std::error::Error;
use std::io::Read;
use std::fs::File;

use image::ImageBuffer;
use image::Rgba;

use nes::NesState;

fn main() {
    let mut window: PistonWindow<GlutinWindow> = WindowSettings::new("RusticNES", [1024, 768])
    .exit_on_esc(true).build().unwrap();

    println!("Welcome to RusticNES");
    println!("Attempting to read mario.nes header");

    let mut file = match File::open("mario.nes") {
        Err(why) => panic!("Couldn't open mario.nes: {}", why.description()),
        Ok(file) => file,
    };
    let mut cartridge = Vec::new();
    // Read the whole damn thing?
    match file.read_to_end(&mut cartridge) {
        Err(why) => panic!("Couldn't read data: {}", why.description()),
        Ok(bytes_read) => println!("Data read successfully: {}", bytes_read),
    };

    let mut nes = NesState::new();
    let nes_header = cartridge::extract_header(&cartridge);
    cartridge::print_header_info(nes_header);
    cartridge::load_from_cartridge(&mut nes, nes_header, &cartridge);

    // Initialize CPU register state for power-up sequence
    nes.registers.a = 0;
    nes.registers.y = 0;
    nes.registers.x = 0;
    nes.registers.s = 0xFD;

    let pc_low = memory::read_byte(&mut nes, 0xFFFC);
    let pc_high = memory::read_byte(&mut nes, 0xFFFD);
    nes.registers.pc = pc_low as u16 + ((pc_high as u16) << 8);

    // Initialized? Let's go!
    let mut exit: bool = false;
    let mut cycles: u32 = 0;

    // "Screen"
    let mut texture_settings = TextureSettings::new()
        .min(texture::Filter::Nearest)
        .mag(texture::Filter::Nearest);

    let mut screen_buffer = ImageBuffer::new(256, 240);
    let mut screen_texture = Texture::from_image(
        &mut window.factory,
        &screen_buffer,
        &texture_settings
    ).unwrap();

    let mut pal_buffer = ImageBuffer::new(16, 32);
    for i in 0 .. 63 {
        let x = i % 16;
        let y = i >> 4;
        let index = (i * 3) as usize;
        pal_buffer.put_pixel(x, y, Rgba { data: [
            palettes::ntsc_pal[index],
            palettes::ntsc_pal[index + 1],
            palettes::ntsc_pal[index + 2],
            255] });
    }
    let mut pal_texture = Texture::from_image(
        &mut window.factory,
        &pal_buffer,
        &texture_settings
    ).unwrap();

    let mut pattern_0_buffer = ImageBuffer::new(128, 128);
    let mut pattern_1_buffer = ImageBuffer::new(128, 128);
    debug::generate_chr_pattern(&nes.ppu.pattern_0, &mut pattern_0_buffer);
    debug::generate_chr_pattern(&nes.ppu.pattern_1, &mut pattern_1_buffer);
    let mut pattern_0_texture = Texture::from_image(&mut window.factory, &pattern_0_buffer,
        &texture_settings).unwrap();
    let mut pattern_1_texture = Texture::from_image(&mut window.factory, &pattern_1_buffer,
        &texture_settings).unwrap();

    let mut nametables_buffer = ImageBuffer::new(512, 480);
    let mut nametables_texture = Texture::from_image(&mut window.factory, &nametables_buffer,
        &texture_settings).unwrap();

    let mut thingy = 0;
    let mut running = false;

    debug::print_program_state(&mut nes);

    while let Some(event) = window.next() {
        if let Some(button) = event.press_args() {
            // Keyboard input here
            if button == Keyboard(Key::R) {
                running = !running;
            }

            if button == Keyboard(Key::Space) {
                // Run one opcode, then debug
                cpu::process_instruction(&mut nes);
                nes.ppu.run_to_cycle(cycles, &mut nes.memory);
                cycles = cycles + 12;

                debug::print_program_state(&mut nes);
            }
        }

        if let Some(_) = event.update_args() {
            // Debug draw some junk
            for x in 0 .. 256 {
                for y in 0 .. 240 {
                    screen_buffer.put_pixel(x, y, Rgba { data: [
                        (x + thingy & 0xFF) as u8,
                        (y + thingy & 0xFF) as u8,
                        ((x ^ y ^ thingy) & 0xFF) as u8,
                        255] });
                }
            }
            screen_texture.update(&mut window.encoder, &screen_buffer);
            debug::generate_nametables(&mut nes.ppu, &mut nametables_buffer);
            nametables_texture.update(&mut window.encoder, &nametables_buffer);

            if running {
                // TODO: Move this into NesEmulator and make it run until vblank
                cpu::process_instruction(&mut nes);
                nes.ppu.run_to_cycle(cycles, &mut nes.memory);
                cycles = cycles + 12;
            }
        }

        window.draw_2d(&event, |context, graphics| {
            clear([0.8; 4], graphics);
            let base_transform = context.transform.scale(2.0, 2.0);
            let pal_transform = base_transform.trans(0.0, 240.0).scale(16.0, 16.0);
            image(&screen_texture, base_transform, graphics);
            image(&pal_texture, pal_transform, graphics);

            let pattern_0_transform = base_transform.trans(256.0, 0.0);
            let pattern_1_transform = base_transform.trans(256.0 + 128.0, 0.0);
            image(&pattern_0_texture, pattern_0_transform, graphics);
            image(&pattern_1_texture, pattern_1_transform, graphics);

            let nametables_transform = base_transform.trans(256.0, 128.0);
            image(&nametables_texture, nametables_transform, graphics);

            thingy = thingy + 1;
        });
    }
}
