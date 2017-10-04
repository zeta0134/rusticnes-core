use cpu;
use cpu::Registers;
use memory::read_byte;
use memory::write_byte;
use nes::NesState;
use opcodes;

pub struct CpuState {
  pub tick: u8,
  pub opcode: u8,
  pub data1: u8,
  pub data2: u8,
}

impl CpuState {
  pub fn new() -> CpuState{
    return CpuState {
      tick: 1,
      opcode: 0,
      data1: 0,
      data2: 0,
    }
  }
}

// ######## Addressing Modes ########

type ReadOpcode = fn(&mut Registers, u8);
type WriteOpcode = fn(&mut Registers) -> u8;
type RmwOpcode = fn(&mut Registers, u8) -> u8;

struct AddressingMode {
  read: fn(&mut NesState, ReadOpcode),
  write: fn(&mut NesState, WriteOpcode),
  rmw: fn(&mut NesState, RmwOpcode),
}

// Note: These will be REMOVED eventually, they are here so we can test code partially.
// Not to be confused with the NOP versions below, which help to group some of the
// processor's unusual behavior with undefined opcodes.
pub fn unimplemented_read(nes: &mut NesState, _: ReadOpcode) {
  nes.registers.pc = nes.registers.pc.wrapping_sub(1);
  cpu::process_instruction(nes);
  nes.cpu.tick = 1;
}

pub fn unimplemented_write(nes: &mut NesState, _: WriteOpcode) {
  nes.registers.pc = nes.registers.pc.wrapping_sub(1);
  cpu::process_instruction(nes);
  nes.cpu.tick = 1;
}

pub fn unimplemented_rmw(nes: &mut NesState, _: RmwOpcode) {
  nes.registers.pc = nes.registers.pc.wrapping_sub(1);
  cpu::process_instruction(nes);
  nes.cpu.tick = 1;
}

// Immediate mode only supports reading data
pub fn immediate_read(nes: &mut NesState, opcode_func: ReadOpcode) {
  let data = nes.cpu.data1;
  opcode_func(&mut nes.registers, data);
  nes.registers.pc = nes.registers.pc.wrapping_add(1);
  nes.cpu.tick = 1;
}

// Absolute mode
pub fn absolute_read(nes: &mut NesState, opcode_func: ReadOpcode) {
  match nes.cpu.tick {
    2 => {
      // data1 is already filled
      nes.registers.pc = nes.registers.pc.wrapping_add(1);}
    3 => {
      let pc = nes.registers.pc;
      nes.cpu.data2 = read_byte(nes, pc);
      nes.registers.pc = nes.registers.pc.wrapping_add(1);}
    4 => {
      let address = ((nes.cpu.data2 as u16) << 8) | (nes.cpu.data1 as u16);
      let data = read_byte(nes, address);
      opcode_func(&mut nes.registers, data);
      nes.cpu.tick = 1;
    },
    _ => ()
  }
}

pub fn absolute_write(nes: &mut NesState, opcode_func: WriteOpcode) {
  match nes.cpu.tick {
    2 => {
      // data1 is already filled
      nes.registers.pc = nes.registers.pc.wrapping_add(1);}
    3 => {
      let pc = nes.registers.pc;
      nes.cpu.data2 = read_byte(nes, pc);
      nes.registers.pc = nes.registers.pc.wrapping_add(1);}
    4 => {
      let address = ((nes.cpu.data2 as u16) << 8) | (nes.cpu.data1 as u16);
      let data = opcode_func(&mut nes.registers);
      write_byte(nes, address, data);
      nes.cpu.tick = 1;
    },
    _ => ()
  }
}


// Called by STA in #imm mode, this has the effect of a two-byte NOP
// which skips the data byte. but still takes just 2 cycles.
pub fn nop_write(nes: &mut NesState, _: WriteOpcode) {
  nes.registers.pc = nes.registers.pc.wrapping_add(1);
  nes.cpu.tick = 1;
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
  }



  // Decode this opcode into its component parts
  let logic_block = nes.cpu.opcode & 0b0000_0011;
  let addressing_mode_index = (nes.cpu.opcode & 0b0001_1100) >> 2;
  let opcode_index = (nes.cpu.opcode & 0b1110_0000) >> 5;

  match logic_block {
    01 => {
      let addressing_mode = match addressing_mode_index {
        // Immediate Mode
        0b010 => AddressingMode{
          read: immediate_read, 
          write: nop_write, 
          rmw: unimplemented_rmw},
        // Absolute Mode
        0x011 => AddressingMode{
          read: absolute_read,
          write: absolute_write,
          rmw: unimplemented_rmw},
        // Not implemented yet
        _ => AddressingMode{
          read: unimplemented_read, 
          write: unimplemented_write, 
          rmw: unimplemented_rmw},
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
      nes.cpu.tick = 1;
    }
  }

}