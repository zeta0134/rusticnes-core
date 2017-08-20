use nes::NesState;

#[derive(Copy, Clone)]
pub struct NesHeader {
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

pub fn extract_header(cartridge: &Vec<u8>) -> NesHeader {
    // See if that worked
    println!("Magic Header: {0} {1} {2} 0x{3:X}", cartridge[0] as char, cartridge[1] as char, cartridge[2] as char, cartridge[3]);

    // Okay, now create an NES struct and massage the data into it
    let mut nes_header: NesHeader = NesHeader {
        prg_rom_size: cartridge[4] as u32 * 16 * 1024,
        chr_rom_size: cartridge[5] as u32 * 8 * 1024,
        mapper_number: (cartridge[6] & 0xF0 >> 4) + cartridge[7] & 0xF0,
        prg_ram_size: cartridge[8] as u32 * 8 * 1024,
        ..Default::default()
    };

    if cartridge[6] & 0x08 != 0 {
        nes_header.four_screen_mirroring = true;
    } else {
        nes_header.horizontal_mirroring = cartridge[6] & 0x01 == 0;
        nes_header.vertical_mirroring   = cartridge[6] & 0x01 != 0;
    }
    nes_header.has_sram = cartridge[6] & 0x02 != 0;
    nes_header.trainer  = cartridge[6] & 0x04 != 0;

    return nes_header;
}

pub fn print_header_info(header: NesHeader) {
    println!("PRG ROM: {0}", header.prg_rom_size);
    println!("CHR ROM: {0}", header.chr_rom_size);
    println!("PRG RAM: {0}", header.prg_ram_size);
    println!("Mapper: {0}", header.mapper_number);
}

pub fn load_from_cartridge(nes: &mut NesState, header: NesHeader, cartridge: &Vec<u8>) {
    let mut offset = 16;
    //let mut trainer = &cartridge[16..16]; //default to empty
    if header.trainer {
        //trainer = &cartridge[offset..(offset + 512)];
        offset = offset + 512;
    }
    let prg_rom_size = (header.prg_rom_size) as usize;
    let prg_rom = &cartridge[offset .. (offset + prg_rom_size)];
    offset = offset + prg_rom_size;

    let chr_rom_size = (header.chr_rom_size) as usize;
    let chr_rom = &cartridge[offset .. (offset + chr_rom_size as usize)];
    //offset = offset + chr_rom_size;

    // Initialize main memory (this is only valid for very simple games)
    if prg_rom_size == 32768 {
        for i in 0 .. prg_rom_size - 1 {
            nes.memory.cart_rom[i] = prg_rom[i];
        }
    } else if prg_rom_size == 16384 {
        for i in 0 .. prg_rom_size - 1 {
            nes.memory.cart_rom[i] = prg_rom[i];
            nes.memory.cart_rom[i + 16384] = prg_rom[i];
        }
    } else {
        println!("UNSUPPORTED PRG SIZE! Will probably break.");
    }

    // Initialize PPU CHR memory (again, only valid for simple mappers)
    for i in 0 .. 0x1000 - 1 {
        nes.ppu.pattern_0[i] = chr_rom[i];
        nes.ppu.pattern_1[i] = chr_rom[0x1000 + i];
    }
}
