use std::ops::Index;
use std::ops::IndexMut;

pub struct CpuMemory {
    // Naive implementation -- a stupid array!
    pub raw: [u8; 0x10000]
}

impl Index<u16> for CpuMemory {
    type Output = u8;

    fn index(&self, address: u16) -> &u8 {
        return &(self.raw[address as usize]);
    }
}

impl IndexMut<u16> for CpuMemory {
    fn index_mut(&mut self, address: u16) -> &mut u8 {
        return &mut (self.raw[address as usize]);
    }
}
