use cpu;
use cpu::Registers;
use memory::read_byte;
use memory::write_byte;
use nes::NesState;

type ReadOpcode = fn(&mut Registers, u8);
type WriteOpcode = fn(&mut Registers) -> u8;
type RmwOpcode = fn(&mut Registers, u8) -> u8;

pub struct AddressingMode {
  pub read: fn(&mut NesState, ReadOpcode),
  pub write: fn(&mut NesState, WriteOpcode),
  pub rmw: fn(&mut NesState, RmwOpcode),
}

// Note: These will be REMOVED eventually, they are here so we can test code partially.
// Not to be confused with the NOP versions below, which help to group some of the
// processor's unusual behavior with undefined opcodes.
pub fn unimplemented_read(nes: &mut NesState, _: ReadOpcode) {
  nes.registers.pc = nes.registers.pc.wrapping_sub(1);
  cpu::process_instruction(nes);
  nes.cpu.tick = 0;
}

pub fn unimplemented_write(nes: &mut NesState, _: WriteOpcode) {
  nes.registers.pc = nes.registers.pc.wrapping_sub(1);
  cpu::process_instruction(nes);
  nes.cpu.tick = 0;
}

pub fn unimplemented_rmw(nes: &mut NesState, _: RmwOpcode) {
  nes.registers.pc = nes.registers.pc.wrapping_sub(1);
  cpu::process_instruction(nes);
  nes.cpu.tick = 0;
}

// Called by STA in #imm mode, this has the effect of a two-byte NOP
// which skips the data byte. but still takes just 2 cycles.
pub fn nop_write(nes: &mut NesState, _: WriteOpcode) {
  nes.registers.pc = nes.registers.pc.wrapping_add(1);
  nes.cpu.tick = 0;
}

pub static UNIMPLEMENTED: AddressingMode = AddressingMode{
  read: unimplemented_read,
  write: unimplemented_write,
  rmw: unimplemented_rmw
};

// Immediate mode only supports reading data
pub fn immediate_read(nes: &mut NesState, opcode_func: ReadOpcode) {
  let data = nes.cpu.data1;
  opcode_func(&mut nes.registers, data);
  nes.registers.pc = nes.registers.pc.wrapping_add(1);
  nes.cpu.tick = 0;
}

pub static IMMEDIATE: AddressingMode = AddressingMode{
  read: immediate_read,
  write: nop_write,
  rmw: unimplemented_rmw
};

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
      nes.cpu.tick = 0;
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
      nes.cpu.tick = 0;
    },
    _ => ()
  }
}

pub static ABSOLUTE: AddressingMode = AddressingMode{
  read: absolute_read,
  write: absolute_write,
  rmw: unimplemented_rmw
};

// Zero Page mode
pub fn zero_page_read(nes: &mut NesState, opcode_func: ReadOpcode) {
  match nes.cpu.tick {
    2 => {
      // data1 is already filled
      nes.registers.pc = nes.registers.pc.wrapping_add(1);}
    3 => {
      let address = nes.cpu.data1 as u16;
      let data = read_byte(nes, address);
      opcode_func(&mut nes.registers, data);
      nes.cpu.tick = 0;
    },
    _ => ()
  }
}

pub fn zero_page_write(nes: &mut NesState, opcode_func: WriteOpcode) {
  match nes.cpu.tick {
    2 => {
      // data1 is already filled
      nes.registers.pc = nes.registers.pc.wrapping_add(1);}
    3 => {
      let address = nes.cpu.data1 as u16;
      let data = opcode_func(&mut nes.registers);
      write_byte(nes, address, data);
      nes.cpu.tick = 0;
    },
    _ => ()
  }
}

pub static ZERO_PAGE: AddressingMode = AddressingMode{
  read: zero_page_read,
  write: zero_page_write,
  rmw: unimplemented_rmw
};

// Zero Page Indexed (X)
pub fn zero_page_indexed_x_read(nes: &mut NesState, opcode_func: ReadOpcode) {
  match nes.cpu.tick {
    2 => {
      // data1 is already filled
      nes.registers.pc = nes.registers.pc.wrapping_add(1);}
    3 => {
      // Dummy read from original address, discarded
      let address = nes.cpu.data1 as u16;
      let _ = read_byte(nes, address);
      nes.cpu.data1 = nes.cpu.data1.wrapping_add(nes.registers.x);
    },
    4 => {
      let effective_address = nes.cpu.data1 as u16;
      let data = read_byte(nes, effective_address);
      opcode_func(&mut nes.registers, data);
      nes.cpu.tick = 0;
    },
    _ => ()
  }
}

pub fn zero_page_indexed_x_write(nes: &mut NesState, opcode_func: WriteOpcode) {
  match nes.cpu.tick {
    2 => {
      // data1 is already filled
      nes.registers.pc = nes.registers.pc.wrapping_add(1);}
    3 => {
      // Dummy read from original address, discarded
      let address = nes.cpu.data1 as u16;
      let _ = read_byte(nes, address);
      nes.cpu.data1 = nes.cpu.data1.wrapping_add(nes.registers.x);
    },
    4 => {
      let effective_address = nes.cpu.data1 as u16;
      let data = opcode_func(&mut nes.registers);
      write_byte(nes, effective_address, data);
      nes.cpu.tick = 0;
    },
    _ => ()
  }
}

pub static ZERO_PAGE_INDEXED_X: AddressingMode = AddressingMode{
  read: zero_page_indexed_x_read,
  write: zero_page_indexed_x_write,
  rmw: unimplemented_rmw
};