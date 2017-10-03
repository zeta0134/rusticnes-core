use cpu;
use cpu::Registers;
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

// ######## Opcodes ########

// Logical inclusive OR
fn ora(registers: &mut Registers, data: u8) {
    registers.a = registers.a | data;
    registers.flags.zero = registers.a == 0;
    registers.flags.negative = registers.a & 0x80 != 0;
}

fn and(registers: &mut Registers, data: u8) {
    registers.a = registers.a & data;
    registers.flags.zero = registers.a == 0;
    registers.flags.negative = registers.a & 0x80 != 0;
}

// Exclusive OR
fn eor(registers: &mut Registers, data: u8) {
    registers.a = registers.a ^ data;
    registers.flags.zero = registers.a == 0;
    registers.flags.negative = registers.a & 0x80 != 0;
}

// Add with Carry
fn adc(registers: &mut Registers, data: u8) {
    let result: u16 = registers.a as u16 + data as u16 + registers.flags.carry as u16;
    registers.flags.carry = result > 0xFF;
    registers.flags.overflow = cpu::overflow(registers.a, data, result as u8);
    registers.a = (result & 0xFF) as u8;
    registers.flags.zero = registers.a == 0;
    registers.flags.negative = registers.a & 0x80 != 0;
}

// Store Accumulator
fn sta(registers: &mut Registers) -> u8 {
    return registers.a
}

// Load Accumulator
fn lda(registers: &mut Registers, data: u8) {
    registers.a = data;
    registers.flags.zero = registers.a == 0;
    registers.flags.negative = registers.a & 0x80 != 0;
}

// Compare Accumulator
fn cmp(registers: &mut Registers, data: u8) {
    registers.flags.carry = registers.a >= data;
    let result: u8 = registers.a.wrapping_sub(data);
    registers.flags.zero = result == 0;
    registers.flags.negative = result & 0x80 != 0;
}

// Subtract with Carry
fn sbc(registers: &mut Registers, data: u8) {
    // Preload the carry into bit 8
    let inverted_data = data ^ 0xFF;
    adc(registers, inverted_data);
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
pub fn unimplemented_read(nes: &mut NesState, opcode_func: ReadOpcode) {
  nes.registers.pc = nes.registers.pc.wrapping_sub(1);
  cpu::process_instruction(nes);
  nes.cpu.tick = 1;
}

pub fn unimplemented_write(nes: &mut NesState, opcode_func: WriteOpcode) {
  nes.registers.pc = nes.registers.pc.wrapping_sub(1);
  cpu::process_instruction(nes);
  nes.cpu.tick = 1;
}

pub fn unimplemented_rmw(nes: &mut NesState, opcode_func: RmwOpcode) {
  nes.registers.pc = nes.registers.pc.wrapping_sub(1);
  cpu::process_instruction(nes);
  nes.cpu.tick = 1;
}

pub fn immediate_read(nes: &mut NesState, opcode_func: ReadOpcode) {
  let data = nes.cpu.data1;
  opcode_func(&mut nes.registers, data);
  nes.registers.pc = nes.registers.pc.wrapping_add(1);
  nes.cpu.tick = 1;
}

// Called by STA in #imm mode, this has the effect of a two-byte NOP
// which skips the data byte. but still takes just 2 cycles.
pub fn nop_write(nes: &mut NesState, opcode_func: WriteOpcode) {
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
        // Not implemented yet
        _ => AddressingMode{
          read: unimplemented_read, 
          write: unimplemented_write, 
          rmw: unimplemented_rmw},
      };

      match opcode_index {
        0b000 => {(addressing_mode.read)(nes, ora)},
        0b001 => {(addressing_mode.read)(nes, and)},
        0b010 => {(addressing_mode.read)(nes, eor)},
        0b011 => {(addressing_mode.read)(nes, adc)},
        0b100 => {(addressing_mode.write)(nes, sta)},
        0b101 => {(addressing_mode.read)(nes, lda)},
        0b110 => {(addressing_mode.read)(nes, cmp)},
        0b111 => {(addressing_mode.read)(nes, sbc)},
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