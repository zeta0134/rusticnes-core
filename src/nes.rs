use cpu;
use cpu::Registers;
use memory::CpuMemory;
use ppu::PpuState;

pub struct NesState {
    pub memory: CpuMemory,
    pub ppu: PpuState,
    pub registers: Registers,
    pub current_cycle: u32,
}

impl NesState {
    pub fn new() -> NesState {
        return NesState {
            memory: CpuMemory::new(),
            ppu: PpuState::new(),
            registers: Registers::new(),
            current_cycle: 0,
        }
    }
}

pub fn step(nes: &mut NesState) {
    cpu::process_instruction(nes);
    nes.ppu.run_to_cycle(nes.current_cycle, &mut nes.memory);
    nes.current_cycle = nes.current_cycle + 12;
}

pub fn run_until_hblank(nes: &mut NesState) {
    let old_scanline = nes.ppu.current_scanline;
    while old_scanline == nes.ppu.current_scanline {
        step(nes);
    }
}

pub fn run_until_vblank(nes: &mut NesState) {
    let old_frame = nes.ppu.current_frame;
    while old_frame == nes.ppu.current_frame {
        step(nes);
    }
}
