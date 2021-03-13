#[derive(Copy, Clone)]
pub struct NsfHeader {
    raw_bytes: [u8; 0x80]
}

const MSDOS_EOF: u8 = 0x1A;

const NSF_MAGIC_N: usize = 0;
const NSF_MAGIC_E: usize = 1;
const NSF_MAGIC_S: usize = 2;
const NSF_MAGIC_M: usize = 3;
const NSF_MAGIC_EOF: usize = 4;
const NSF_VERSION: usize = 4;

impl NsfHeader {
    pub fn from(raw_bytes: &[u8]) -> NsfHeader {
        let mut header = NsfHeader {
            raw_bytes: [0u8; 0x80],
        };
        header.raw_bytes.copy_from_slice(&raw_bytes[0..0x80]);
        return header;
    }

    pub fn magic_header_valid(&self) -> bool {
        return 
            self.raw_bytes[NSF_MAGIC_N] as char == 'N' &&
            self.raw_bytes[NSF_MAGIC_E] as char == 'E' &&
            self.raw_bytes[NSF_MAGIC_S] as char == 'S' &&
            self.raw_bytes[NSF_MAGIC_M] as char == 'M' &&
            self.raw_bytes[NSF_MAGIC_EOF] == MSDOS_EOF;
    }

    pub fn version_number(&self) -> u8 {
        return self.raw_bytes[NSF_VERSION];
    }


}
