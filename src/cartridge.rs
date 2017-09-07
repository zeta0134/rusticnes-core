use mmc::mapper::*;
use mmc::axrom::AxRom;
use mmc::nrom::Nrom;
use mmc::mmc1::Mmc1;
use nes::NesState;

// iNES 1.0 Header. Flags and decoding documented here. 2.0 is not yet supported.
// https://wiki.nesdev.com/w/index.php/INES
#[derive(Copy, Clone)]
pub struct NesHeader {
  pub prg_rom_size: usize,
  pub chr_rom_size: usize,
  pub mapper_number: u8,
  pub prg_ram_size: usize,
  pub has_chr_ram: bool,

  // Flags 6
  pub mirroring: Mirroring,
  pub has_sram: bool,
  pub trainer: bool,

  // There are many other flags in the iNES header, but they're unsupported,
  // so they're ommitted here.
}

impl Default for NesHeader {
    fn default() -> NesHeader {
        NesHeader {
            prg_rom_size: 0,
            chr_rom_size: 0,
            mapper_number: 0,
            prg_ram_size: 0,

            has_sram: false,
            has_chr_ram: false,
            trainer: false,
            mirroring: Mirroring::Vertical,
        }
    }
}

pub fn extract_header(cartridge: &Vec<u8>) -> NesHeader {
    // TODO: Detect if the magic header doesn't match and either bail, or print a warning.
    println!("Magic Header: {0}{1}{2} 0x{3:X}", cartridge[0] as char, cartridge[1] as char, cartridge[2] as char, cartridge[3]);

    let prg_size = cartridge[4] as usize * 16 * 1024;
    let chr_size = cartridge[5] as usize * 8 * 1024;
    let mapper_low = (cartridge[6] & 0b1111_0000) >> 4;
    let mapper_high = cartridge[7] & 0b1111_0000;
    let mapper_number = mapper_low | mapper_high;
    let ram_size = cartridge[8] as usize * 8 * 1024;

    let mut nes_header: NesHeader = NesHeader {
        prg_rom_size: prg_size,
        chr_rom_size: chr_size,
        mapper_number: mapper_number,
        prg_ram_size: ram_size,
        ..Default::default()
    };

    let four_screen_mirroring = cartridge[6] & 0b0000_1000 != 0;
    let vertical_mirroring = cartridge[6] & 0b0000_0001 == 0;

    if four_screen_mirroring {
        nes_header.mirroring = Mirroring::FourScreen;
    } else {
        if vertical_mirroring {
            nes_header.mirroring = Mirroring::Vertical;
        } else {
            nes_header.mirroring = Mirroring::Horizontal;
        }
    }

    nes_header.has_sram = cartridge[6] & 0b0000_0010 != 0;
    nes_header.trainer  = cartridge[6] & 0b0000_0100 != 0;

    return nes_header;
}

pub fn print_header_info(header: NesHeader) {
    println!("PRG ROM: {0}", header.prg_rom_size);
    println!("CHR ROM: {0}", header.chr_rom_size);
    println!("PRG RAM: {0}", header.prg_ram_size);
    println!("Mapper: {0}", header.mapper_number);
}

pub fn load_from_cartridge(nes_header: NesHeader, cartridge: &Vec<u8>) -> Box<Mapper> {
    let mut offset = 16;
    let mut header = nes_header;

    if header.trainer {
        offset = offset + 512;
    }

    let prg_rom_size = (header.prg_rom_size) as usize;
    let prg_rom = &cartridge[offset .. (offset + prg_rom_size)];
    offset = offset + prg_rom_size;

    let chr_rom_size = (header.chr_rom_size) as usize;
    let mut chr_rom = &cartridge[offset .. (offset + chr_rom_size as usize)];

    if header.chr_rom_size == 0 {
        header.chr_rom_size = 8 * 1024;
        header.has_chr_ram = true;
    }

    let mapper: Box<Mapper> = match header.mapper_number {
        0 => Box::new(Nrom::new(header, chr_rom, prg_rom)),
        1 => Box::new(Mmc1::new(header, chr_rom, prg_rom)),
        7 => Box::new(AxRom::new(header, chr_rom, prg_rom)),
        _ => {
            println!("Undefined mapper: {}", header.mapper_number);
            println!("Will proceed as though this is NROM, which will LIKELY NOT WORK.");
            return Box::new(Nrom::new(header, chr_rom, prg_rom));
        }
    };

    return mapper;
}
