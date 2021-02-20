// Parses and manages header fields and data blobs for iNES 1.0 and 2.0
// Details here: https://wiki.nesdev.com/w/index.php/INES
// And here: https://wiki.nesdev.com/w/index.php/NES_2.0#Default_Expansion_Device
// iNES 2.0 is always preferred when detected.

use std::io::Read;
use std::error::Error;
use std::fmt;


use mmc::mapper::Mirroring;

#[derive(Debug)]
pub enum INesError {
    InvalidHeader,
    Unimplemented,
    ReadError{reason: String}
}

impl Error for INesError {}

impl fmt::Display for INesError  {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            INesError::InvalidHeader => {write!(f, "Invalid iNES Header")},
            INesError::Unimplemented => {write!(f, "Unimplemented (Lazy programmers!!1)")},
            INesError::ReadError{reason} => {write!(f, "Error reading cartridge: {}", reason)}
        }
    }
}

impl From<std::io::Error> for INesError {
    fn from(error: std::io::Error) -> Self {
        return INesError::ReadError{reason: error.to_string()};
    }
}

#[derive(Copy, Clone)]
pub struct INesHeader {
    raw_bytes: [u8; 16]
}

impl INesHeader {
    pub fn from(raw_bytes: &[u8]) -> INesHeader {
        let mut header = INesHeader {
            raw_bytes: [0u8; 16],
        };
        header.raw_bytes.copy_from_slice(&raw_bytes[0..16]);
        return header;
    }

    pub fn magic_header_valid(&self) -> bool {
        // Constant $4E $45 $53 $1A ("NES" followed by MS-DOS end-of-file)
        return 
            self.raw_bytes[0] as char == 'N' &&
            self.raw_bytes[1] as char == 'E' &&
            self.raw_bytes[2] as char == 'S' &&
            self.raw_bytes[3] == 0x1A;
    }

    pub fn version(&self) -> u8 {
        // A file is a NES 2.0 ROM image file if it begins with "NES<EOF>" (same as iNES) and, 
        // additionally, the byte at offset 7 has bit 2 clear and bit 3 set:
        if self.raw_bytes[7] == 0x08 {
            return 2;
        }
        if self.magic_header_valid() {
            return 1;
        }
        return 0;
    }

    fn _prg_size_ines1(&self) -> usize {
        return self.raw_bytes[4] as usize * 16 * 1024;
    }

    fn _prg_size_ines2(&self) -> usize {
        // https://wiki.nesdev.com/w/index.php/NES_2.0#PRG-ROM_Area
        let lsb = self.raw_bytes[4];
        let msb = self.raw_bytes[9] & 0b0000_1111;
        if msb == 0xF {
            // exponent-multiplier mode
            //  ++++----------- Header byte 9 D0..D3
            //  |||| ++++-++++- Header byte 4
            //  D~BA98 7654 3210
            //  --------------
            //  1111 EEEE EEMM
            //  |||| ||++- Multiplier, actual value is MM*2+1 (1,3,5,7)
            //  ++++-++--- Exponent (2^E), 0-63

            let multiplier = ((lsb & 0b0000_0011) * 2 + 1) as usize;
            let exponent = ((lsb & 0b1111_1100) >> 2) as u32;
            let base: usize = 2;
            return base.pow(exponent) * multiplier;
        } else {
            // simple mode
            return ((msb as usize) << 8) + (lsb as usize) * 16 * 1024;
        }
    }

    pub fn prg_size(&self) -> usize {
        return match self.version() {
            1 => self._prg_size_ines1(),
            2 => self._prg_size_ines2(),
            _ => 0
        }
    }

    fn _chr_size_ines1(&self) -> usize {
        return self.raw_bytes[5] as usize * 8 * 1024;
    }

    fn _chr_size_ines2(&self) -> usize {
        // https://wiki.nesdev.com/w/index.php/NES_2.0#PRG-ROM_Area
        let lsb = self.raw_bytes[5];
        let msb = (self.raw_bytes[9] & 0b1111_0000) >> 4;
        if msb == 0xF {
            // exponent-multiplier mode
            //  ++++----------- Header byte 9 D0..D3
            //  |||| ++++-++++- Header byte 4
            //  D~BA98 7654 3210
            //  --------------
            //  1111 EEEE EEMM
            //  |||| ||++- Multiplier, actual value is MM*2+1 (1,3,5,7)
            //  ++++-++--- Exponent (2^E), 0-63

            let multiplier = ((lsb & 0b0000_0011) * 2 + 1) as usize;
            let exponent = ((lsb & 0b1111_1100) >> 2) as u32;
            let base: usize = 2;
            return base.pow(exponent) * multiplier;
        } else {
            // simple mode
            return ((msb as usize) << 8) + (lsb as usize) * 8 * 1024;
        }
    }

    pub fn chr_size(&self) -> usize {
        return match self.version() {
            1 => self._chr_size_ines1(),
            2 => self._chr_size_ines2(),
            _ => 0
        }
    }



    // https://wiki.nesdev.com/w/index.php/INES#Flags_6

    pub fn mirroring(&self) -> Mirroring {
        if self.raw_bytes[6] & 0b0000_1000 != 0 {
            return Mirroring::FourScreen;
        }
        if self.raw_bytes[6] & 0b0000_0001 != 0 {
            return Mirroring::Vertical;
        }
        return Mirroring::Horizontal;
    }

    pub fn persistent_sram(&self) -> bool {
        return self.raw_bytes[6] & 0b0000_0010 != 0;
    }

    pub fn has_trainer(&self) -> bool {
        return self.raw_bytes[6] & 0b0000_0100 != 0;
    }
}

#[derive(Clone)]
pub struct INesCartridge {
    // Internal strategy is to store each major chunk of the file as
    // raw data bytes, and then reinterpret these on the fly based
    // on the header bytes when accessed.
    header: INesHeader,
    trainer: Vec<u8>,
    prg: Vec<u8>,
    chr: Vec<u8>,
    misc_rom: Vec<u8>,
}

impl INesCartridge {
    pub fn from_bytes(file_data: &[u8]) -> Result<INesCartridge, INesError> {
        let mut file_reader = file_data;
        
        let mut header_bytes = [0u8; 16];
        file_reader.read_exact(&mut header_bytes)?;

        let header = INesHeader::from(&header_bytes);
        if !header.magic_header_valid() {
            return Err(INesError::InvalidHeader);
        }

        let trainer_size = if header.has_trainer() {512} else {0};
        let mut trainer: Vec<u8> = Vec::with_capacity(trainer_size);
        file_reader.read_exact(&mut trainer)?;

        let mut prg: Vec<u8> = Vec::with_capacity(header.prg_size());
        file_reader.read_exact(&mut prg)?;

        let mut chr: Vec<u8> = Vec::with_capacity(header.chr_size());
        file_reader.read_exact(&mut chr)?;

        // If there is any remaining data at this point, it becomes misc_rom and,
        // currently, has no other special handling
        let mut misc: Vec<u8> = Vec::new();
        file_reader.read_to_end(&mut misc)?;

        return Ok(INesCartridge {
            header: header,
            trainer: trainer,
            prg: prg,
            chr: chr,
            misc_rom: misc
        });
    }
}