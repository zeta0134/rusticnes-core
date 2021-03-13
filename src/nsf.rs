#[derive(Copy, Clone)]
pub struct NsfHeader {
    raw_bytes: [u8; 0x80]
}

const MSDOS_EOF: u8 = 0x1A;

const NSF_MAGIC_N: usize = 0x000;
const NSF_MAGIC_E: usize = 0x001;
const NSF_MAGIC_S: usize = 0x002;
const NSF_MAGIC_M: usize = 0x003;
const NSF_MAGIC_EOF: usize = 0x004;
const NSF_VERSION: usize = 0x005;
const NSF_TOTAL_SONGS: usize = 0x006;
const NSF_STARTING_SONG: usize = 0x007;
const NSF_LOAD_ADDR: usize = 0x008;
const NSF_INIT_ADDR: usize = 0x00A;
const NSF_PLAY_ADDR: usize = 0x00C;
//const NSF_SONG_NAME: usize = 0x00E;
//const NSF_ARTIST_NAME: usize = 0x02E;
//const NSF_COPYRIGHT_HOLDER: usize = 0x04E;
const NSF_NTSC_PLAY_SPEED: usize = 0x06E;
//const NSF_BANK_INIT: usize = 0x070;
const NSF_PAL_PLAY_SPEED: usize = 0x078;

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

    pub fn total_songs(&self) -> u8 {
        return self.raw_bytes[NSF_TOTAL_SONGS];
    }

    pub fn starting_song(&self) -> u8 {
        return self.raw_bytes[NSF_STARTING_SONG];
    }

    pub fn _word(&self, offset: usize) -> u16 {
        let addr_low =   self.raw_bytes[offset + 0] as u16;
        let addr_high = (self.raw_bytes[offset + 1] as u16) << 8;
        return addr_low + addr_high;
    }

    pub fn load_address(&self) -> u16 {
        return self._word(NSF_LOAD_ADDR);
    }

    pub fn init_address(&self) -> u16 {
        return self._word(NSF_INIT_ADDR);
    }

    pub fn play_address(&self) -> u16 {
        return self._word(NSF_PLAY_ADDR);
    }

    /* strings are complicated, let's skip them for now */

    pub fn ntsc_playback_speed(&self) -> u16 {
        return self._word(NSF_NTSC_PLAY_SPEED);
    }

    pub fn pal_playback_speed(&self) -> u16 {
        return self._word(NSF_PAL_PLAY_SPEED);
    }




}
