mod cpu;

use std::error::Error;
use std::io::Read;
use std::fs::File;

struct NesHeader {
  prg_rom_size: u32,
  chr_rom_size: u32,
  mapper_number: u8,
  prg_ram_size: u32,
}

fn main() {
    println!("Hello, world!");
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

    // See if that worked
    println!("Magic Header: {0} {1} {2} 0x{3:X}", cartridge[0] as char, cartridge[1] as char, cartridge[2] as char, cartridge[3]);

    // Okay, now create an NES struct and massage the data into it
    let nes_header: NesHeader = NesHeader {
        prg_rom_size: cartridge[4] as u32 * 16 * 1024,
        chr_rom_size: cartridge[5] as u32 * 8 * 1024,
        mapper_number: (cartridge[6] & 0xF0 >> 4) + cartridge[7] & 0xF0,
        prg_ram_size: cartridge[8] as u32 * 8 * 1024,
    };

    println!("PRG ROM: {0}", nes_header.prg_rom_size);
    println!("CHR ROM: {0}", nes_header.chr_rom_size);
    println!("PRG RAM: {0}", nes_header.prg_ram_size);
    println!("Mapper: {0}", nes_header.mapper_number);

    let data: u8 = 255;
    let offset: i8 = data as i8;
    let mut address: u16 = 100;
    address = address.wrapping_add(offset as u16);
    println!("TEST - Adjusted Address: {0}", address);
}
