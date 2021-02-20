// Parses and manages header fields and data blobs for iNES 1.0 and 2.0
// Details here: https://wiki.nesdev.com/w/index.php/INES
// And here: https://wiki.nesdev.com/w/index.php/NES_2.0#Default_Expansion_Device
// iNES 2.0 is always preferred when detected.

use std::io::Read;

use mmc::mapper::Mirroring;

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
    pub fn from_bytes(file_data: &[u8]) -> Result<INesCartridge, String> {
        let mut file_reader = file_data;
        
        let mut header_bytes = [0u8; 16];
        file_reader.read_exact(&mut header_bytes).map_err(|e| e.to_string())?;

        let header = INesHeader::from(&header_bytes);
        if !header.magic_header_valid() {
            return Err("iNES signature invalid".to_string());
        }

        let mut trainer: Vec<u8> = Vec::new();
        if header.has_trainer() {
            trainer.resize(512, 0);
            file_reader.read_exact(&mut trainer).map_err(|e| e.to_string())?;
        }






        // */

        return Err("Unimplemented".to_string());
    }
}