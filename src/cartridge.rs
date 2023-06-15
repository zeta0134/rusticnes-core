use crate::mmc::mapper::*;
use crate::mmc::action53::Action53;
use crate::mmc::axrom::AxRom;
use crate::mmc::bnrom::BnRom;
use crate::mmc::cnrom::CnRom;
use crate::mmc::fme7::Fme7;
use crate::mmc::gxrom::GxRom;
use crate::mmc::ines31::INes31;
use crate::mmc::mmc1::Mmc1;
use crate::mmc::mmc3::Mmc3;
use crate::mmc::mmc5::Mmc5;
use crate::mmc::n163::Namco163;
use crate::mmc::nrom::Nrom;
use crate::mmc::nsf::NsfMapper;
use crate::mmc::pxrom::PxRom;
use crate::mmc::uxrom::UxRom;
use crate::mmc::vrc6::Vrc6;

use crate::ines::INesCartridge;
use crate::nsf::NsfFile;

use std::io::Read;

fn mapper_from_ines(ines: INesCartridge) -> Result<Box<dyn Mapper>, String> {
    let mapper_number = ines.header.mapper_number();

    let mapper: Box<dyn Mapper> = match mapper_number {
        0 => Box::new(Nrom::from_ines(ines)?),
        1 => Box::new(Mmc1::from_ines(ines)?),
        2 => Box::new(UxRom::from_ines(ines)?),
        3 => Box::new(CnRom::from_ines(ines)?),
        4 => Box::new(Mmc3::from_ines(ines)?),
        5 => Box::new(Mmc5::from_ines(ines)?),
        7 => Box::new(AxRom::from_ines(ines)?),
        9 => Box::new(PxRom::from_ines(ines)?),
        19 => Box::new(Namco163::from_ines(ines)?),
        24 => Box::new(Vrc6::from_ines(ines)?),
        26 => Box::new(Vrc6::from_ines(ines)?),
        28 => Box::new(Action53::from_ines(ines)?),
        31 => Box::new(INes31::from_ines(ines)?),
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

pub fn mapper_from_reader(file_reader: &mut dyn Read) -> Result<Box<dyn Mapper>, String> {
    let mut entire_file = Vec::new();
    match file_reader.read_to_end(&mut entire_file) {
        Ok(_) => {/* proceed normally */},
        Err(e) => {
            return Err(format!("Failed to read any data at all, giving up.{}\n", e));
        }
    }

    let mut errors = String::new();
    match INesCartridge::from_reader(&mut entire_file.as_slice()) {
        Ok(ines) => {return mapper_from_ines(ines);},
        Err(e) => {errors += format!("ines: {}\n", e).as_str()}
    }

    match NsfFile::from_reader(&mut entire_file.as_slice()) {
        Ok(nsf) => {return Ok(Box::new(NsfMapper::from_nsf(nsf)?));},
        Err(e) => {errors += format!("nsf: {}\n", e).as_str()}
    }

    return Err(format!("Unable to open file as any known type, giving up.\n{}", errors));
}

pub fn mapper_from_file(file_data: &[u8]) -> Result<Box<dyn Mapper>, String> {
    let mut file_reader = file_data;
    return mapper_from_reader(&mut file_reader);
}