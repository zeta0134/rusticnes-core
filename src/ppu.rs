use memory::CpuMemory;

pub fn update(memory: &mut CpuMemory, cycles: u32) {
    // Timing? Nah, just signal vblank every frame.
    memory.ppu_status = (memory.ppu_status & 0x7F) + 0x80;
}
