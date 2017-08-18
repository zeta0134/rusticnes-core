use cpu::Registers;
use memory::CpuMemory;
use ppu::PpuState;

pub struct NesState {
    pub memory: CpuMemory,
    pub ppu: PpuState,
    pub registers: Registers,
}

impl NesState {
    pub fn new() -> NesState {
        return NesState {
            memory: CpuMemory::new(),
            ppu: PpuState::new(),
            registers: Registers::new(),
        }
    }
}
