// FDS: an archival format for the Famicom Disk System, detailed here:
// https://www.nesdev.org/wiki/FDS_file_format

use std::io::Read;
use std::error::Error;
use std::fmt;

#[derive(Copy, Clone)]
pub struct FdsHeader {
    raw_bytes: [u8; 16]
}

const MSDOS_EOF: u8 = 0x1A;

const FDS_MAGIC_F: usize = 0x000;
const FDS_MAGIC_D: usize = 0x001;
const FDS_MAGIC_S: usize = 0x002;
const FDS_MAGIC_EOF: usize = 0x003;
const FDS_DISK_SIDES: usize = 0x004;

impl FdsHeader {
    pub fn from(raw_bytes: &[u8]) -> FdsHeader {
        let mut header = FdsHeader {
            raw_bytes: [0u8; 16],
        };
        header.raw_bytes.copy_from_slice(&raw_bytes[0..16]);
        return header;
    }

    pub fn magic_header_valid(&self) -> bool {
        return 
            self.raw_bytes[FDS_MAGIC_F] as char == 'F' &&
            self.raw_bytes[FDS_MAGIC_D] as char == 'D' &&
            self.raw_bytes[FDS_MAGIC_S] as char == 'S' &&
            self.raw_bytes[FDS_MAGIC_EOF] == MSDOS_EOF;
    }

    pub fn num_disk_sides(&self) -> usize {
        return self.raw_bytes[FDS_DISK_SIDES] as usize;
    }
}

#[derive(Debug)]
pub enum FdsError {
    InvalidHeader,    
    ReadError{reason: String}
}

impl Error for FdsError {}

impl fmt::Display for FdsError  {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FdsError::InvalidHeader => {write!(f, "Invalid FDS Header")},
            FdsError::ReadError{reason} => {write!(f, "Error reading disk image: {}", reason)}
        }
    }
}

impl From<std::io::Error> for FdsError {
    fn from(error: std::io::Error) -> Self {
        return FdsError::ReadError{reason: error.to_string()};
    }
}

#[derive(Clone)]
pub struct FdsFile {
    pub header: FdsHeader,
    pub disk_sides: Vec<Vec<u8>>,
}

impl FdsFile {
    pub fn from_reader(file_reader: &mut dyn Read) -> Result<FdsFile, FdsError> {
        // Read in the *whole* file at once. We need to try several different headers
        // because that's a thing, so we can't assume any particular one is present.
        let mut fds_data: Vec<u8> = Vec::new();
        file_reader.read_to_end(&mut fds_data)?;

        let mut disk_sides: Vec<Vec<u8>> = Vec::new();

        // First try the 16-byte header originating in fwNES
        let header = FdsHeader::from(&fds_data[0..16]);
        if header.magic_header_valid() {
            for i in 0 .. header.num_disk_sides() {
                let start = 16 + (i * 65500);
                let end = 16 + ((i+1) * 65500);
                if end > fds_data.len() {
                    return Err(FdsError::ReadError{reason: "Unexpected end of file!".to_string()});        
                }
                let disk_side_bytes = Vec::from(&fds_data[start..end]);
                disk_sides.push(disk_side_bytes);
            }
            return Ok(FdsFile {
                header: header,
                disk_sides: disk_sides,
            });
        }

        // Second, see if the first 15 bytes correspond to the start of info block 1. If they do, this is
        // likely a raw dump. Assume disk sides as a multiple of 65500 bytes and complain if we have anything else
        let verification_string = "\x01*NINTENDO-HVC*";
        let candidate_string = std::str::from_utf8(&fds_data[0..15]).expect("invalid utf8-sequence");
        if candidate_string.eq(verification_string) {
            for i in 0 .. fds_data.len() / 65500 {
                let start = i * 65500;
                let end = (i+1) * 65500;
                if end > fds_data.len() {
                    return Err(FdsError::ReadError{reason: "Unexpected end of file!".to_string()});        
                }
                let disk_side_bytes = Vec::from(&fds_data[start..end]);
                disk_sides.push(disk_side_bytes);
            }
            return Ok(FdsFile {
                header: header,
                disk_sides: disk_sides,
            });
        }

        return Err(FdsError::InvalidHeader);
    }
}
