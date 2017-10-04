// Documentation for this 6502 implementation came from many sources, but the following
// two guides served as the primary inspiration:
// http://www.llx.com/~nparker/a2/opcodes.html - For opcode decoding structure
// http://nesdev.com/6502_cpu.txt - for information on cycle timings for each addressing mode

use addressing;
use cpu;
use memory::read_byte;
use nes::NesState;
use opcodes;

pub struct CpuState {
  pub tick: u8,
  pub opcode: u8,
  pub data1: u8,
  pub data2: u8,
  pub temp_address: u16,
}

impl CpuState {
  pub fn new() -> CpuState{
    return CpuState {
      tick: 0,
      opcode: 0,
      data1: 0,
      data2: 0,
      temp_address: 0,
    }
  }
}

pub fn run_one_clock(nes: &mut NesState) {
  nes.cpu.tick += 1;

  // Universal behavior for every opcode
  if nes.cpu.tick == 1 {
    // Fetch opcode from memory
    let pc = nes.registers.pc;
    nes.cpu.opcode = read_byte(nes, pc);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
    return; // all done
  }

  // Every instruction performs this read, regardless of whether
  // the data is used.
  if nes.cpu.tick == 2 {
    // Fetch data byte from memory
    let pc = nes.registers.pc;
    nes.cpu.data1 = read_byte(nes, pc);
  }

  // Decode this opcode into its component parts
  let logic_block = nes.cpu.opcode & 0b0000_0011;
  let addressing_mode_index = (nes.cpu.opcode & 0b0001_1100) >> 2;
  let opcode_index = (nes.cpu.opcode & 0b1110_0000) >> 5;

  match logic_block {
    0b01 => {
      let addressing_mode = match addressing_mode_index {
        // Zero Page Mode
        0b000 => &addressing::INDEXED_INDIRECT_X,
        0b001 => &addressing::ZERO_PAGE,
        0b010 => &addressing::IMMEDIATE,
        0b011 => &addressing::ABSOLUTE,
        0b100 => &addressing::INDIRECT_INDEXED_Y,
        0b101 => &addressing::ZERO_PAGE_INDEXED_X,
        0b110 => &addressing::ABSOLUTE_INDEXED_Y,
        0b111 => &addressing::ABSOLUTE_INDEXED_X,

        // Not implemented yet
        _ => &addressing::UNIMPLEMENTED,
      };

      match opcode_index {
        0b000 => {(addressing_mode.read)(nes, opcodes::ora)},
        0b001 => {(addressing_mode.read)(nes, opcodes::and)},
        0b010 => {(addressing_mode.read)(nes, opcodes::eor)},
        0b011 => {(addressing_mode.read)(nes, opcodes::adc)},
        0b100 => {(addressing_mode.write)(nes, opcodes::sta)},
        0b101 => {(addressing_mode.read)(nes, opcodes::lda)},
        0b110 => {(addressing_mode.read)(nes, opcodes::cmp)},
        0b111 => {(addressing_mode.read)(nes, opcodes::sbc)},
        _ => ()
      };
    },
    _ => {
      // We don't have this block implemented! Fall back to old behavior.
      nes.registers.pc = nes.registers.pc.wrapping_sub(1);
      cpu::process_instruction(nes);
      nes.cpu.tick = 0;
    }
  }
}