use cpu;
use cpu::Registers;
use memory::read_byte;
use memory::write_byte;
use nes::NesState;

type ReadOpcode = fn(&mut Registers, u8);
type WriteOpcode = fn(&mut Registers) -> u8;
type ModifyOpcode = fn(&mut Registers, u8) -> u8;

pub struct AddressingMode {
  pub read: fn(&mut NesState, ReadOpcode),
  pub write: fn(&mut NesState, WriteOpcode),
  pub modify: fn(&mut NesState, ModifyOpcode),
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

pub fn unimplemented_modify(nes: &mut NesState, _: ModifyOpcode) {
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
  modify: unimplemented_modify
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
  modify: unimplemented_modify
};

// Absolute mode
pub fn absolute_read(nes: &mut NesState, opcode_func: ReadOpcode) {
  match nes.cpu.tick {
    2 => {
      // data1 is already filled
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
    },
    3 => {
      let pc = nes.registers.pc;
      nes.cpu.data2 = read_byte(nes, pc);
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
    },
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
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
    },
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
  modify: unimplemented_modify
};

// Zero Page mode
pub fn zero_page_read(nes: &mut NesState, opcode_func: ReadOpcode) {
  match nes.cpu.tick {
    2 => {
      // data1 is already filled
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
    },
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
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
    },
    3 => {
      let address = nes.cpu.data1 as u16;
      let data = opcode_func(&mut nes.registers);
      write_byte(nes, address, data);
      nes.cpu.tick = 0;
    },
    _ => ()
  }
}

pub fn zero_page_modify(nes: &mut NesState, opcode_func: ModifyOpcode) {
  match nes.cpu.tick {
    2 => {
      // data1 is already filled
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
    },
    3 => {
      // Read from the zero-page address
      let address = nes.cpu.data1 as u16;
      nes.cpu.data2 = read_byte(nes, address);
    },
    4 => {
      // Dummy write to the zero-page address
      let address = nes.cpu.data1 as u16;
      let data = nes.cpu.data2;
      write_byte(nes, address, data);

      // Perform the opcode on the data
      let result = opcode_func(&mut nes.registers, data);
      nes.cpu.data2 = result;
    },
    5 => {
      // Write the result to the effective address
      let address = nes.cpu.data1 as u16;
      let data = nes.cpu.data2;
      write_byte(nes, address, data);
      nes.cpu.tick = 0;
    },
    _ => ()
  }
}

pub static ZERO_PAGE: AddressingMode = AddressingMode{
  read: zero_page_read,
  write: zero_page_write,
  modify: unimplemented_modify
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
  modify: unimplemented_modify
};

pub fn indexed_indirect_x_read(nes: &mut NesState, opcode_func: ReadOpcode) {
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
      // Read low byte of indirect address
      let effective_address = nes.cpu.data1 as u16;
      nes.cpu.temp_address = read_byte(nes, effective_address) as u16;
      nes.cpu.data1 = nes.cpu.data1.wrapping_add(1);
    },
    5 => {
      // Read high byte of indirect address
      let effective_address = nes.cpu.data1 as u16;
      nes.cpu.temp_address = ((read_byte(nes, effective_address) as u16) << 8) | nes.cpu.temp_address;
    },
    6 => {
      let temp_address = nes.cpu.temp_address;
      let data = read_byte(nes, temp_address);
      opcode_func(&mut nes.registers, data);
      nes.cpu.tick = 0;
    },
    _ => {}
  }
}

pub fn indexed_indirect_x_write(nes: &mut NesState, opcode_func: WriteOpcode) {
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
      // Read low byte of indirect address
      let effective_address = nes.cpu.data1 as u16;
      nes.cpu.temp_address = read_byte(nes, effective_address) as u16;
      nes.cpu.data1 = nes.cpu.data1.wrapping_add(1);
    },
    5 => {
      // Read high byte of indirect address
      let effective_address = nes.cpu.data1 as u16;
      nes.cpu.temp_address = ((read_byte(nes, effective_address) as u16) << 8) | nes.cpu.temp_address;
    },
    6 => {
      let temp_address = nes.cpu.temp_address;
      let data = opcode_func(&mut nes.registers);
      write_byte(nes, temp_address, data);
      nes.cpu.tick = 0;
    },
    _ => {}
  }
}

pub static INDEXED_INDIRECT_X: AddressingMode = AddressingMode{
  read: indexed_indirect_x_read,
  write: indexed_indirect_x_write,
  modify: unimplemented_modify
};

pub fn indirect_indexed_y_read(nes: &mut NesState, opcode_func: ReadOpcode) {
  match nes.cpu.tick {
    2 => {
      // data1 is already filled
      nes.registers.pc = nes.registers.pc.wrapping_add(1);}
    3 => {
      // Read low byte of indirect address
      let pointer_low = nes.cpu.data1 as u16;
      nes.cpu.temp_address = read_byte(nes, pointer_low) as u16;
      nes.cpu.data1 = nes.cpu.data1.wrapping_add(1);
    },
    4 => {
      // Read high byte of indirect address
      let pointer_high = nes.cpu.data1 as u16;
      nes.cpu.temp_address = ((read_byte(nes, pointer_high) as u16) << 8) | nes.cpu.temp_address;
    },
    5 => {
      // Add Y to LOW BYTE of the effective address
      // Accuracy note: technically this occurs in cycle 4, but as it has no effect on emulation, I've
      // moved it to the beginning of cycle 5, as it makes the early escape detection more straightforward.
      let low_byte = (nes.cpu.temp_address & 0xFF) + (nes.registers.y as u16);
      nes.cpu.temp_address = (nes.cpu.temp_address & 0xFF00) | (low_byte & 0xFF);
      // Read from this new address
      let temp_address = nes.cpu.temp_address;
      let data = read_byte(nes, temp_address);
      // If the new address doesn't need adjustment, run the opcode now and bail early, intentionally
      // skipping cycle 6
      if low_byte <= 0xFF {
        opcode_func(&mut nes.registers, data);
        nes.cpu.tick = 0;
      } else {
        // Fix the high byte of the address by adding 1 to it
        nes.cpu.temp_address = nes.cpu.temp_address.wrapping_add(0x100);  
      }
    },
    6 => {
      // Read from the final address and run this opcode
      let temp_address = nes.cpu.temp_address;
      let data = read_byte(nes, temp_address);
      opcode_func(&mut nes.registers, data);
      nes.cpu.tick = 0;
    },
    _ => {}
  }
}

pub fn indirect_indexed_y_write(nes: &mut NesState, opcode_func: WriteOpcode) {
  match nes.cpu.tick {
    2 => {
      // data1 is already filled
      nes.registers.pc = nes.registers.pc.wrapping_add(1);}
    3 => {
      // Read low byte of indirect address
      let pointer_low = nes.cpu.data1 as u16;
      nes.cpu.temp_address = read_byte(nes, pointer_low) as u16;
      nes.cpu.data1 = nes.cpu.data1.wrapping_add(1);
    },
    4 => {
      // Read high byte of indirect address
      let pointer_high = nes.cpu.data1 as u16;
      nes.cpu.temp_address = ((read_byte(nes, pointer_high) as u16) << 8) | nes.cpu.temp_address;
    },
    5 => {
      // Add Y to LOW BYTE of the effective address
      // Accuracy note: technically this occurs in cycle 4, but as it has no effect on emulation, I've
      // moved it to the beginning of cycle 5, as it makes the early escape detection more straightforward.
      let low_byte = (nes.cpu.temp_address & 0xFF) + (nes.registers.y as u16);
      nes.cpu.temp_address = (nes.cpu.temp_address & 0xFF00) | (low_byte & 0xFF);
      let temp_address = nes.cpu.temp_address;
      // Dummy read from the new address before it is fixed
      let _ = read_byte(nes, temp_address);
      if low_byte > 0xFF {
        // Fix the high byte of the address by adding 1 to it
        nes.cpu.temp_address = nes.cpu.temp_address.wrapping_add(0x100);
      }
    },
    6 => {
      // Write the result of this opcode to the final address
      let temp_address = nes.cpu.temp_address;
      let data = opcode_func(&mut nes.registers);
      write_byte(nes, temp_address, data);
      nes.cpu.tick = 0;
    },
    _ => {}
  }
}

pub static INDIRECT_INDEXED_Y: AddressingMode = AddressingMode{
  read: indirect_indexed_y_read,
  write: indirect_indexed_y_write,
  modify: unimplemented_modify
};

pub fn absolute_indexed_x_read(nes: &mut NesState, opcode_func: ReadOpcode) {
  match nes.cpu.tick {
    2 => {
      // data1 is already filled with the low byte
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
    }
    3 => {
      // read the high byte into data 2
      let pc = nes.registers.pc;
      nes.cpu.data2 = read_byte(nes, pc);
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
      nes.cpu.temp_address = ((nes.cpu.data2 as u16) << 8) | (nes.cpu.data1 as u16);
    },
    4 => {
      // Add X to LOW BYTE of the effective address
      // Accuracy note: technically this occurs in cycle 3
      let low_byte = (nes.cpu.temp_address & 0xFF) + (nes.registers.x as u16);
      nes.cpu.temp_address = (nes.cpu.temp_address & 0xFF00) | (low_byte & 0xFF);
      // Read from this new address
      let temp_address = nes.cpu.temp_address;
      let data = read_byte(nes, temp_address);
      // If the new address doesn't need adjustment, run the opcode now and bail early, intentionally
      // skipping cycle 5
      if low_byte <= 0xFF {
        opcode_func(&mut nes.registers, data);
        nes.cpu.tick = 0;
      } else {
        // Fix the high byte of the address by adding 1 to it
        nes.cpu.temp_address = nes.cpu.temp_address.wrapping_add(0x100);  
      }
    },
    5 => {
      // Read from the final address and run this opcode
      let temp_address = nes.cpu.temp_address;
      let data = read_byte(nes, temp_address);
      opcode_func(&mut nes.registers, data);
      nes.cpu.tick = 0;
    },
    _ => ()
  }
}

pub fn absolute_indexed_x_write(nes: &mut NesState, opcode_func: WriteOpcode) {
  match nes.cpu.tick {
    2 => {
      // data1 is already filled with the low byte
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
    }
    3 => {
      // read the high byte into data 2
      let pc = nes.registers.pc;
      nes.cpu.data2 = read_byte(nes, pc);
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
      nes.cpu.temp_address = ((nes.cpu.data2 as u16) << 8) | (nes.cpu.data1 as u16);
    },
    4 => {
      // Add X to LOW BYTE of the effective address
      // Accuracy note: technically this occurs in cycle 4, but as it has no effect on emulation, I've
      // moved it to the beginning of cycle 5, as it makes the early escape detection more straightforward.
      let low_byte = (nes.cpu.temp_address & 0xFF) + (nes.registers.x as u16);
      nes.cpu.temp_address = (nes.cpu.temp_address & 0xFF00) | (low_byte & 0xFF);
      let temp_address = nes.cpu.temp_address;
      // Dummy read from the new address before it is fixed
      let _ = read_byte(nes, temp_address);
      if low_byte > 0xFF {
        // Fix the high byte of the address by adding 1 to it
        nes.cpu.temp_address = nes.cpu.temp_address.wrapping_add(0x100);
      }
    },
    5 => {
      // Write the result of this opcode to the final address
      let temp_address = nes.cpu.temp_address;
      let data = opcode_func(&mut nes.registers);
      write_byte(nes, temp_address, data);
      nes.cpu.tick = 0;
    },
    _ => {}
  }
}

pub static ABSOLUTE_INDEXED_X: AddressingMode = AddressingMode{
  read: absolute_indexed_x_read,
  write: absolute_indexed_x_write,
  modify: unimplemented_modify
};

pub fn absolute_indexed_y_read(nes: &mut NesState, opcode_func: ReadOpcode) {
  match nes.cpu.tick {
    2 => {
      // data1 is already filled with the low byte
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
    }
    3 => {
      // read the high byte into data 2
      let pc = nes.registers.pc;
      nes.cpu.data2 = read_byte(nes, pc);
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
      nes.cpu.temp_address = ((nes.cpu.data2 as u16) << 8) | (nes.cpu.data1 as u16);
    },
    4 => {
      // Add X to LOW BYTE of the effective address
      // Accuracy note: technically this occurs in cycle 3
      let low_byte = (nes.cpu.temp_address & 0xFF) + (nes.registers.y as u16);
      nes.cpu.temp_address = (nes.cpu.temp_address & 0xFF00) | (low_byte & 0xFF);
      // Read from this new address
      let temp_address = nes.cpu.temp_address;
      let data = read_byte(nes, temp_address);
      // If the new address doesn't need adjustment, run the opcode now and bail early, intentionally
      // skipping cycle 5
      if low_byte <= 0xFF {
        opcode_func(&mut nes.registers, data);
        nes.cpu.tick = 0;
      } else {
        // Fix the high byte of the address by adding 1 to it
        nes.cpu.temp_address = nes.cpu.temp_address.wrapping_add(0x100);  
      }
    },
    5 => {
      // Read from the final address and run this opcode
      let temp_address = nes.cpu.temp_address;
      let data = read_byte(nes, temp_address);
      opcode_func(&mut nes.registers, data);
      nes.cpu.tick = 0;
    },
    _ => ()
  }
}

pub fn absolute_indexed_y_write(nes: &mut NesState, opcode_func: WriteOpcode) {
  match nes.cpu.tick {
    2 => {
      // data1 is already filled with the low byte
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
    }
    3 => {
      // read the high byte into data 2
      let pc = nes.registers.pc;
      nes.cpu.data2 = read_byte(nes, pc);
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
      nes.cpu.temp_address = ((nes.cpu.data2 as u16) << 8) | (nes.cpu.data1 as u16);
    },
    4 => {
      // Add X to LOW BYTE of the effective address
      // Accuracy note: technically this occurs in cycle 4, but as it has no effect on emulation, I've
      // moved it to the beginning of cycle 5, as it makes the early escape detection more straightforward.
      let low_byte = (nes.cpu.temp_address & 0xFF) + (nes.registers.y as u16);
      nes.cpu.temp_address = (nes.cpu.temp_address & 0xFF00) | (low_byte & 0xFF);
      let temp_address = nes.cpu.temp_address;
      // Dummy read from the new address before it is fixed
      let _ = read_byte(nes, temp_address);
      if low_byte > 0xFF {
        // Fix the high byte of the address by adding 1 to it
        nes.cpu.temp_address = nes.cpu.temp_address.wrapping_add(0x100);
      }
    },
    5 => {
      // Write the result of this opcode to the final address
      let temp_address = nes.cpu.temp_address;
      let data = opcode_func(&mut nes.registers);
      write_byte(nes, temp_address, data);
      nes.cpu.tick = 0;
    },
    _ => {}
  }
}

pub static ABSOLUTE_INDEXED_Y: AddressingMode = AddressingMode{
  read: absolute_indexed_y_read,
  write: absolute_indexed_y_write,
  modify: unimplemented_modify
};

