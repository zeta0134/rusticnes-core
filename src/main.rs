extern crate image;
extern crate pancurses;
extern crate piston_window;

use piston_window::*;

mod cpu;
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

struct NesHeader {
  prg_rom_size: u32,
  chr_rom_size: u32,
  mapper_number: u8,
  prg_ram_size: u32,

  // Flags 6
  horizontal_mirroring: bool,
  vertical_mirroring: bool,
  has_sram: bool,
  trainer: bool,
  four_screen_mirroring: bool,
}

impl Default for NesHeader {
    fn default() -> NesHeader {
        NesHeader {
            prg_rom_size: 0,
            chr_rom_size: 0,
            mapper_number: 0,
            prg_ram_size: 0,

            horizontal_mirroring: false,
            vertical_mirroring: false,
            has_sram: false,
            trainer: false,
            four_screen_mirroring: false,
        }
    }
}

fn print_program_state(console: &mut pancurses::Window, nes: &mut NesState) {
    let registers = nes.registers;
    console.printw(&format!("A: 0x{:02X} X: 0x{:02X} Y: 0x{:02X}\n", registers.a, registers.x, registers.y));
    console.printw(&format!("PC: 0x{:02X} S: 0x{:02X}\n", registers.pc, registers.s));
    console.printw(&format!("Flags: nv  dzic\n"));
    console.printw(&format!("       {:b}{:b}  {:b}{:b}{:b}{:b}\n",
        registers.flags.negative as u8,
        registers.flags.overflow as u8,
        registers.flags.decimal as u8,
        registers.flags.zero as u8,
        registers.flags.interrupts_disabled as u8,
        registers.flags.carry as u8,
    ));
    console.printw("\nMemory @ Program Counter\n");
    // print out the next 8 bytes or so from the program counter
    let mut pc = registers.pc;
    for i in 1 .. 8 {
        console.printw(&format!("0x{:04X}: 0x{:02X}\n", pc, memory::passively_read_byte(nes, pc)));
        pc = pc.wrapping_add(1);
    }
}

fn main() {
    let mut console = pancurses::initscr();

    let mut window: PistonWindow = WindowSettings::new("RusticNES", [1024, 768])
    .exit_on_esc(true).build().unwrap();

    console.printw("Hello, world!");
    console.printw("Attempting to read mario.nes header");

    let mut file = match File::open("mario.nes") {
        Err(why) => panic!("Couldn't open mario.nes: {}", why.description()),
        Ok(file) => file,
    };
    let mut cartridge = Vec::new();
    // Read the whole damn thing?
    match file.read_to_end(&mut cartridge) {
        Err(why) => panic!("Couldn't read data: {}", why.description()),
        Ok(bytes_read) => console.printw(&format!("Data read successfully: {}", bytes_read)),
    };

    // See if that worked
    console.printw(&format!("Magic Header: {0} {1} {2} 0x{3:X}", cartridge[0] as char, cartridge[1] as char, cartridge[2] as char, cartridge[3]));

    // Okay, now create an NES struct and massage the data into it
    let mut nes_header: NesHeader = NesHeader {
        prg_rom_size: cartridge[4] as u32 * 16 * 1024,
        chr_rom_size: cartridge[5] as u32 * 8 * 1024,
        mapper_number: (cartridge[6] & 0xF0 >> 4) + cartridge[7] & 0xF0,
        prg_ram_size: cartridge[8] as u32 * 8 * 1024,
        ..Default::default()
    };

    console.printw(&format!("PRG ROM: {0}", nes_header.prg_rom_size));
    console.printw(&format!("CHR ROM: {0}", nes_header.chr_rom_size));
    console.printw(&format!("PRG RAM: {0}", nes_header.prg_ram_size));
    console.printw(&format!("Mapper: {0}", nes_header.mapper_number));

    if cartridge[6] & 0x08 != 0 {
        nes_header.four_screen_mirroring = true;
    } else {
        nes_header.horizontal_mirroring = cartridge[6] & 0x01 == 0;
        nes_header.vertical_mirroring   = cartridge[6] & 0x01 != 0;
    }
    nes_header.has_sram = cartridge[6] & 0x02 != 0;
    nes_header.trainer  = cartridge[6] & 0x04 != 0;

    let mut offset = 16;
    let mut trainer = &cartridge[16..16]; //default to empty
    if nes_header.trainer {
        trainer = &cartridge[offset..(offset + 512)];
        offset = offset + 512;
    }
    let prg_rom_size = (nes_header.prg_rom_size) as usize;
    let prg_rom = &cartridge[offset .. (offset + prg_rom_size)];
    offset = offset + prg_rom_size;

    let chr_rom_size = (nes_header.chr_rom_size) as usize;
    let chr_rom = &cartridge[offset .. (offset + chr_rom_size as usize)];
    offset = offset + chr_rom_size;

    let mut nes = NesState::new();

    // Initialize main memory (this is only valid for very simple games)
    for i in 0 .. 32768 - 1 {
        nes.memory.cart_rom[i] = prg_rom[i];
    }

    // Initialize PPU CHR memory (again, only valid for simple mappers)
    for i in 0 .. 0x1000 - 1 {
        nes.ppu.pattern_0[i] = chr_rom[i];
        nes.ppu.pattern_1[i] = chr_rom[0x1000 + i];
    }

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

    let mut thingy = 0;

    //while !exit {
    while let Some(event) = window.next() {
        screen_texture.update(&mut window.encoder, &screen_buffer);
        window.draw_2d(&event, |context, graphics| {
            console.clear();
            print_program_state(&mut console, &mut nes);
            console.refresh();
            //let input = console.getch();
            //if input == Some(pancurses::Input::Character('q')) {
            //    exit = true;
            //}
            cpu::process_instruction(&mut nes);
            nes.ppu.run_to_cycle(cycles, &mut nes.memory);
            cycles = cycles + 12;

            clear([0.8; 4], graphics);

            for x in 0 .. 256 {
                for y in 0 .. 240 {
                    screen_buffer.put_pixel(x, y, Rgba { data: [
                        (x + thingy & 0xFF) as u8,
                        (y + thingy & 0xFF) as u8,
                        ((x ^ y ^ thingy) & 0xFF) as u8,
                        255] });
                }
            }
            let base_transform = context.transform.scale(2.0, 2.0);
            let pal_transform = base_transform.trans(256.0, 0.0).scale(16.0, 16.0);
            image(&screen_texture, base_transform, graphics);
            image(&pal_texture, pal_transform, graphics);
            thingy = thingy + 1;
        });
    }

    pancurses::endwin();
}
