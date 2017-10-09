use apu::ApuState;
use cycle_cpu;
use cpu::Registers;
use cycle_cpu::CpuState;
use memory::CpuMemory;
use ppu::PpuState;
use mmc::mapper::Mapper;

pub struct NesState {
    pub apu: ApuState,
    pub cpu: CpuState,
    pub memory: CpuMemory,
    pub ppu: PpuState,
    pub registers: Registers,
    pub master_clock: u64,
    pub p1_input: u8,
    pub p1_data: u8,
    pub p2_input: u8,
    pub p2_data: u8,
    pub input_latch: bool,
    pub mapper: Box<Mapper>,
}

impl NesState {
    pub fn new(m: Box<Mapper>) -> NesState {
        return NesState {
            apu: ApuState::new(),
            cpu: CpuState::new(),
            memory: CpuMemory::new(),
            ppu: PpuState::new(),
            registers: Registers::new(),
            master_clock: 0,
            p1_input: 0,
            p1_data: 0,
            p2_input: 0,
            p2_data: 0,
            input_latch: false,
            mapper: m,
        }
    }
}

pub fn cycle(nes: &mut NesState) {
    //cpu::process_instruction(nes);
    cycle_cpu::run_one_clock(nes);
    nes.master_clock = nes.master_clock + 12;
    nes.ppu.run_to_cycle(&mut *nes.mapper, nes.master_clock);
    nes.apu.clock_apu(&mut *nes.mapper);
}

pub fn step(nes: &mut NesState) {
    // Start this instruction
    cycle(nes);
    let mut i = 0;
    while nes.cpu.tick >= 1 && i < 10 {
        // Continue until this instruction terminates or halts
        cycle(nes);
        i += 1;
    }
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
