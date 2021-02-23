use mmc::mapper::*;
use mmc::axrom::AxRom;
use mmc::bnrom::BnRom;
use mmc::cnrom::CnRom;
use mmc::fme7::Fme7;
use mmc::gxrom::GxRom;
//use mmc::ines31::INes31;
use mmc::nrom::Nrom;
//use mmc::pxrom::PxRom;
//use mmc::mmc1::Mmc1;
//use mmc::mmc3::Mmc3;
//use mmc::mmc5::Mmc5;
//use mmc::uxrom::UxRom;

use ines::INesCartridge;

// iNES 1.0 Header. Flags and decoding documented here. 2.0 is not yet supported.
// https://wiki.nesdev.com/w/index.php/INES
/*#[derive(Copy, Clone)]
pub struct NesHeader {
  pub magic: [u8; 4],
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
}*/

/*impl Default for NesHeader {
    fn default() -> NesHeader {
        NesHeader {
            magic: [0u8; 4],
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
}*/

/*impl NesHeader {
    pub fn print_info(&self) {
        // TODO: Detect if the magic header doesn't match and either bail, or print a warning.
        println!("Magic Header: {0}{1}{2} 0x{3:X}", self.magic[0] as char, self.magic[1] as char, self.magic[2] as char, self.magic[3]);

        println!("PRG ROM: {0}", self.prg_rom_size);
        println!("CHR ROM: {0}", self.chr_rom_size);
        println!("PRG RAM: {0}", self.prg_ram_size);
        println!("Mapper: {0}", self.mapper_number);
    }

    pub fn magic_is_valid(&self) -> bool {
        return 
          self.magic[0] as char == 'N' &&
          self.magic[1] as char == 'E' &&
          self.magic[2] as char == 'S' &&
          self.magic[3] == 0x1A;
    }
}*/

/*pub fn extract_header(cartridge: &[u8]) -> NesHeader {
    let prg_size = cartridge[4] as usize * 16 * 1024;
    let chr_size = cartridge[5] as usize * 8 * 1024;
    let mapper_low = (cartridge[6] & 0b1111_0000) >> 4;
    let mapper_high = cartridge[7] & 0b1111_0000;
    let mapper_number = mapper_low | mapper_high;
    let ram_size = cartridge[8] as usize * 8 * 1024;

    let mut nes_header: NesHeader = NesHeader {
        magic: [cartridge[0], cartridge[1], cartridge[2], cartridge[3]],
        prg_rom_size: prg_size,
        chr_rom_size: chr_size,
        mapper_number: mapper_number,
        prg_ram_size: ram_size,
        ..Default::default()
    };

    let four_screen_mirroring = (cartridge[6] & 0b0000_1000) != 0;
    let vertical_mirroring = (cartridge[6] & 0b0000_0001) != 0;

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
}*/

/*pub fn load_from_cartridge(nes_header: NesHeader, cartridge: &[u8]) -> Result<Box<dyn Mapper>, String> {
    let mut offset = 16;
    let mut header = nes_header;

    if header.trainer {
        offset = offset + 512;
    }

    let prg_rom_size = (header.prg_rom_size) as usize;
    let prg_rom = &cartridge[offset .. (offset + prg_rom_size)];
    offset = offset + prg_rom_size;

    let chr_rom_size = (header.chr_rom_size) as usize;
    let chr_rom = &cartridge[offset .. (offset + chr_rom_size as usize)];

    if header.chr_rom_size == 0 {
        header.chr_rom_size = 8 * 1024;
        header.has_chr_ram = true;
    }

    if header.prg_ram_size == 0 {
        // For iNES 1.0, assume a minimum PRG RAM size of 8k on boards which support varying sizes
        header.prg_ram_size = 8 * 1024;
    }

    let mapper: Box<dyn Mapper> = match header.mapper_number {
        0 => Box::new(Nrom::new(header, chr_rom, prg_rom)),
        1 => Box::new(Mmc1::new(header, chr_rom, prg_rom)),
        2 => Box::new(UxRom::new(header, chr_rom, prg_rom)),
        3 => Box::new(CnRom::new(header, chr_rom, prg_rom)),
        4 => Box::new(Mmc3::new(header, chr_rom, prg_rom)),
        5 => Box::new(Mmc5::new(header, chr_rom, prg_rom)),
        7 => Box::new(AxRom::new(header, chr_rom, prg_rom)),
        9 => Box::new(PxRom::new(header, chr_rom, prg_rom)),
        31 => Box::new(INes31::new(header, chr_rom, prg_rom)),
        34 => Box::new(BnRom::new(header, chr_rom, prg_rom)),
        66 => Box::new(GxRom::new(header, chr_rom, prg_rom)),
        69 => Box::new(Fme7::new(header, chr_rom, prg_rom)),
        _ => {
            return Err(format!("Unsupported iNES mapper: {}", header.mapper_number));
        }
    };

    return Ok(mapper);
}*/

fn mapper_from_ines(ines: INesCartridge) -> Result<Box<dyn Mapper>, String> {
    let mapper_number = ines.header.mapper_number();

    let mapper: Box<dyn Mapper> = match mapper_number {
        0 => Box::new(Nrom::from_ines(ines)?),
        //1 => Box::new(Mmc1::new(header, chr_rom, prg_rom)),
        //2 => Box::new(UxRom::new(header, chr_rom, prg_rom)),
        3 => Box::new(CnRom::from_ines(ines)?),
        //4 => Box::new(Mmc3::new(header, chr_rom, prg_rom)),
        //5 => Box::new(Mmc5::new(header, chr_rom, prg_rom)),
        7 => Box::new(AxRom::from_ines(ines)?),
        //9 => Box::new(PxRom::new(header, chr_rom, prg_rom)),
        //31 => Box::new(INes31::new(header, chr_rom, prg_rom)),
        34 => Box::new(BnRom::from_ines(ines)?),
        66 => Box::new(GxRom::from_ines(ines)?),
        69 => Box::new(Fme7::from_ines(ines)?),
        _ => {
            return Err(format!("Unsupported iNES mapper: {}", ines.header.mapper_number()));
        }
    };

    println!("Successfully loaded mapper: {}", mapper_number);

    return Ok(mapper);
}

pub fn mapper_from_file(file_data: &[u8]) -> Result<Box<dyn Mapper>, String> {
    let mut file_reader = file_data;
    let mut errors = String::new();
    match INesCartridge::from_reader(&mut file_reader) {
        Ok(ines) => {return mapper_from_ines(ines);},
        Err(e) => {errors += format!("ines: {}\n", e).as_str()}
    }

    return Err(format!("Unable to open file as any known type, giving up.\n{}", errors));
}
