use cpu;
use memory::write_byte;
use memory::read_byte;
use nes::NesState;

pub struct CpuState {
  pub tick: u8,
  pub opcode: u8,
  pub data1: u8,
}

impl CpuState {
  pub fn new() -> CpuState{
    return CpuState {
      tick: 1,
      opcode: 0,
      data1: 0,
    }
  }
}



pub fn run_one_clock(nes: &mut NesState) {
  let tick = nes.cpu.tick;
  nes.cpu.tick += 1;

  // Universal behavior for every opcode
  if tick == 1 {
    // Fetch opcode from memory
    let pc = nes.registers.pc;
    nes.cpu.opcode = read_byte(nes, pc);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
    return; // all done
  }

  // Every instruction performs this read, regardless of whether
  // the data is used.
  if tick == 2 {
    // Fetch data byte from memory
    let pc = nes.registers.pc;
    nes.cpu.data1 = read_byte(nes, pc);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
  }

  // Decode this opcode into its component parts
  let logic_block = nes.cpu.opcode & 0b0000_0011;
  let addressing_mode = (nes.cpu.opcode & 0b0001_1100) >> 2;
  let opcode = (nes.cpu.opcode & 0b1110_0000) >> 5;

  match logic_block {
    _ => {
      // We don't have this opcode implemented! Fall back to old behavior.
      nes.registers.pc = nes.registers.pc.wrapping_sub(2);
      cpu::process_instruction(nes);
      nes.cpu.tick = 1;
    }
  }

}