use cpu;
use cpu::Registers;
use memory::CpuMemory;
use ppu::PpuState;

pub struct NesState {
    pub memory: CpuMemory,
    pub ppu: PpuState,
    pub registers: Registers,
    pub current_cycle: u32,
    pub p1_input: u8,
    pub p1_data: u8,
    pub p2_input: u8,
    pub p2_data: u8,
    pub input_latch: bool,
}

impl NesState {
    pub fn new() -> NesState {
        return NesState {
            memory: CpuMemory::new(),
            ppu: PpuState::new(),
            registers: Registers::new(),
            current_cycle: 0,
            p1_input: 0,
            p1_data: 0,
            p2_input: 0,
            p2_data: 0,
            input_latch: false,
        }
    }
}

pub fn step(nes: &mut NesState) {
    cpu::process_instruction(nes);
    nes.ppu.run_to_cycle(nes.current_cycle);
    nes.current_cycle = nes.current_cycle + 12;
}

pub fn run_until_hblank(nes: &mut NesState) {
    let old_scanline = nes.ppu.current_scanline;
    while old_scanline == nes.ppu.current_scanline {
        step(nes);
    }
}

pub fn run_until_vblank(nes: &mut NesState) {
    while nes.ppu.current_scanline == 242 {
        step(nes);
    }
    while nes.ppu.current_scanline != 242 {
        step(nes);
    }
}
