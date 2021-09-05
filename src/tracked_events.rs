#[derive(Clone)]
pub enum TrackedEvent {
    NullEvent,
    CpuRead{scanline: u16, cycle: u16, address: u16, data: u8},
    CpuWrite{scanline: u16, cycle: u16, address: u16, data: u8},
}

