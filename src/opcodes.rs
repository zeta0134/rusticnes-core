use addressing;
use cpu;
use cpu::Registers;
use nes::NesState;
use memory::read_byte;
use memory::write_byte;

// Logical inclusive OR
pub fn ora(registers: &mut Registers, data: u8) {
  registers.a = registers.a | data;
  registers.flags.zero = registers.a == 0;
  registers.flags.negative = registers.a & 0x80 != 0;
}

pub fn and(registers: &mut Registers, data: u8) {
  registers.a = registers.a & data;
  registers.flags.zero = registers.a == 0;
  registers.flags.negative = registers.a & 0x80 != 0;
}

// Exclusive OR
pub fn eor(registers: &mut Registers, data: u8) {
  registers.a = registers.a ^ data;
  registers.flags.zero = registers.a == 0;
  registers.flags.negative = registers.a & 0x80 != 0;
}

// Add with Carry
pub fn adc(registers: &mut Registers, data: u8) {
  let result: u16 = registers.a as u16 + data as u16 + registers.flags.carry as u16;
  registers.flags.carry = result > 0xFF;
  registers.flags.overflow = cpu::overflow(registers.a, data, result as u8);
  registers.a = (result & 0xFF) as u8;
  registers.flags.zero = registers.a == 0;
  registers.flags.negative = registers.a & 0x80 != 0;
}

// Store Accumulator
pub fn sta(registers: &mut Registers) -> u8 {
  return registers.a
}

// Load Accumulator
pub fn lda(registers: &mut Registers, data: u8) {
  registers.a = data;
  registers.flags.zero = registers.a == 0;
  registers.flags.negative = registers.a & 0x80 != 0;
}

// Compare Accumulator
pub fn cmp(registers: &mut Registers, data: u8) {
  registers.flags.carry = registers.a >= data;
  let result: u8 = registers.a.wrapping_sub(data);
  registers.flags.zero = result == 0;
  registers.flags.negative = result & 0x80 != 0;
}

// Subtract with Carry
pub fn sbc(registers: &mut Registers, data: u8) {
  // Preload the carry into bit 8
  let inverted_data = data ^ 0xFF;
  adc(registers, inverted_data);
}

// Arithmetic Shift Left
pub fn asl(registers: &mut Registers, data: u8) -> u8 {
    registers.flags.carry = data & 0x80 != 0;
    let result = (data & 0x7F) << 1;
    registers.flags.zero = result == 0;
    registers.flags.negative = result & 0x80 != 0;
    return result;
}

// Logical shift right
pub fn lsr(registers: &mut Registers, data: u8) -> u8 {
    registers.flags.carry = data & 0x1 != 0;
    let result: u8 = data >> 1;
    registers.flags.zero = result == 0;
    registers.flags.negative = false;
    return result;
}

// Rotate left
pub fn rol(registers: &mut Registers, data: u8) -> u8 {
    let old_carry = registers.flags.carry;
    registers.flags.carry = (data & 0x80) != 0;
    let result = (data << 1) | (old_carry as u8);
    registers.flags.zero = result == 0;
    registers.flags.negative = (result & 0x80) != 0;
    return result;
}

// Rotate Right
pub fn ror(registers: &mut Registers, data: u8) -> u8 {
    let old_carry = registers.flags.carry;
    registers.flags.carry = (data & 0x01) != 0;
    let result = (data >> 1) | ((old_carry as u8) << 7);
    registers.flags.zero = result == 0;
    registers.flags.negative = (result & 0x80) != 0;
    return result;
}

// Store X
pub fn stx(registers: &mut Registers) -> u8 {
    return registers.x
}

// Load X
pub fn ldx(registers: &mut Registers, data: u8) {
    registers.x = data;
    registers.flags.zero = registers.x == 0;
    registers.flags.negative = registers.x & 0x80 != 0;
}

// Increment Memory
pub fn inc(registers: &mut Registers, data: u8) -> u8 {
    let result: u8 = data.wrapping_add(1);
    registers.flags.zero = result == 0;
    registers.flags.negative = result & 0x80 != 0;
    return result;
}

// Decrement Memory
pub fn dec(registers: &mut Registers, data: u8) -> u8 {
    let result: u8 = data.wrapping_sub(1);
    registers.flags.zero = result == 0;
    registers.flags.negative = result & 0x80 != 0;
    return result;
}

// Decrement X
pub fn dex(registers: &mut Registers) {
    registers.x = registers.x.wrapping_sub(1);
    registers.flags.zero = registers.x == 0;
    registers.flags.negative = registers.x & 0x80 != 0;
}

// Transfer A -> X
pub fn tax(registers: &mut Registers) {
    registers.x = registers.a;
    registers.flags.zero = registers.x == 0;
    registers.flags.negative = registers.x & 0x80 != 0;
}

// Transfer X -> A
pub fn txa(registers: &mut Registers) {
    registers.a = registers.x;
    registers.flags.zero = registers.a == 0;
    registers.flags.negative = registers.a & 0x80 != 0;
}

// Transfer X -> S
pub fn txs(registers: &mut Registers) {
    registers.s = registers.x;
}

// Transfer S -> X
pub fn tsx(registers: &mut Registers) {
    registers.x = registers.s;
    registers.flags.zero = registers.x == 0;
    registers.flags.negative = registers.x & 0x80 != 0;
}

// Load Y
pub fn ldy(registers: &mut Registers, data: u8) {
    registers.y = data;
    registers.flags.zero = registers.y == 0;
    registers.flags.negative = registers.y & 0x80 != 0;
}

// Store Y
pub fn sty(registers: &mut Registers) -> u8 {
    return registers.y
}

// Compare Y
pub fn cpy(registers: &mut Registers, data: u8) {
    registers.flags.carry = registers.y >= data;
    let result: u8 = registers.y.wrapping_sub(data);
    registers.flags.zero = result == 0;
    registers.flags.negative = result & 0x80 != 0;
}

// Compare X
pub fn cpx(registers: &mut Registers, data: u8) {
    registers.flags.carry = registers.x >= data;
    let result: u8 = registers.x.wrapping_sub(data);
    registers.flags.zero = result == 0;
    registers.flags.negative = result & 0x80 != 0;
}

// Bit Test
pub fn bit(registers: &mut Registers, data: u8) {
    let result: u8 = registers.a & data;
    registers.flags.zero = result == 0;
    registers.flags.overflow = data & 0x40 != 0;
    registers.flags.negative = data & 0x80 != 0;
}

// NOP (implemented with an implied signature, for consistency)
pub fn nop(_: &mut Registers) {
}

// NOP - Read and Write variants
pub fn nop_read(_: &mut Registers, _: u8) {
}

pub fn nop_write(_: &mut Registers) -> u8 {
  return 0; // Meant to be discarded
}

pub fn nop_modify(_: &mut Registers, data: u8) -> u8 {
  return data;
}

// Branch
pub fn branch(nes: &mut NesState) {
  // Note: the documentation for branch timing, located at http://nesdev.com/6502_cpu.txt includes
  // the first cycle of the next instruction. Thus, when bailing here, we set nes.cpu.tick to 1
  // instead of zero, so we don't execute the fetch step a second time when we're done.
  match nes.cpu.tick {
    2 => {
      let pc = nes.registers.pc;
      nes.cpu.data1 = read_byte(nes, pc);
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
    },
    3 => {
      // Fetch opcode of next instruction
      let pc = nes.registers.pc;
      let opcode = read_byte(nes, pc);

      // Determine if branch is to be taken
      let flag_index = (nes.cpu.opcode & 0b1100_0000) >> 6;
      let flag_cmp =   (nes.cpu.opcode & 0b0010_0000) != 0;
      let branch_taken = match flag_index {
        0b00 => flag_cmp == nes.registers.flags.negative,
        0b01 => flag_cmp == nes.registers.flags.overflow,
        0b10 => flag_cmp == nes.registers.flags.carry,
        0b11 => flag_cmp == nes.registers.flags.zero,
        _ => {/* Impossible */ false},
      };

      if branch_taken {
        // Add the relative offset to PC, but store ONLY the low byte
        let result = nes.registers.pc.wrapping_add((nes.cpu.data1 as i8) as u16);
        nes.registers.pc = (nes.registers.pc & 0xFF00) | (result & 0xFF);

        // store high byte of result into data2 for further processing
        nes.cpu.data2 = (result >> 8) as u8;
      } else {
        // Actually use that opcode read, increment PC, and bail
        nes.cpu.opcode = opcode;
        nes.registers.pc = nes.registers.pc.wrapping_add(1);
        nes.cpu.tick = 1;
      }
    },
    4 => {
      // Fetch opcode of next instruction, possibly from the wrong address
      let pc = nes.registers.pc;
      let opcode = read_byte(nes, pc);

      if (nes.registers.pc & 0xFF00) != ((nes.cpu.data2 as u16) << 8) {
        nes.registers.pc = (nes.registers.pc & 0xFF) | ((nes.cpu.data2 as u16) << 8);
      } else {
        // PCH didn't need fixing, so bail early, using the opcode we read
        nes.cpu.opcode = opcode;
        nes.registers.pc = nes.registers.pc.wrapping_add(1);
        nes.cpu.tick = 1;
      }
    },
    5 => {
      // Fetch opcode of next instruction
      let pc = nes.registers.pc;
      nes.cpu.opcode = read_byte(nes, pc);
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
      // Finally done
      nes.cpu.tick = 1;
    }
    _ => ()
  }
}

// This isn't strictly an opcode, but it's very similar to the BRK instruction below,
// and is the routine the processer runs when an interrupt occurs. Close enough.
pub fn service_interrupt(nes: &mut NesState) {
  match nes.cpu.tick {
    1 => {
      let pc = nes.registers.pc;
      let _ = read_byte(nes, pc);
      nes.cpu.opcode = 0x00; // Force next opcode to BRK
    },
    2 => {
      // Fetch data byte from memory (and discard it)
      let pc = nes.registers.pc;
      nes.cpu.data1 = read_byte(nes, pc);
    },
    3 => {
      let pc_high = ((nes.registers.pc & 0xFF00) >> 8) as u8;
      cpu::push(nes, pc_high);
    },
    4 => {
      let pc_low =  (nes.registers.pc & 0x00FF) as u8;
      cpu::push(nes, pc_low);
    },
    5 => {
      // At this point, NMI always takes priority, otherwise we run
      // an IRQ
      if nes.cpu.nmi_requested {
        nes.cpu.nmi_requested = false;
        nes.cpu.temp_address = 0xFFFA;
      } else {
        nes.cpu.temp_address = 0xFFFE;
      }
      let status_byte = cpu::status_as_byte(&mut nes.registers, false);
      cpu::push(nes, status_byte);
    },
    6 => {
      // Read PCL from interrupt vector
      let interrupt_vector = nes.cpu.temp_address;
      nes.registers.pc = (nes.registers.pc & 0xFF00) | read_byte(nes, interrupt_vector) as u16;
      // Disable IRQ handling (to be re-enabled by software, usually during RTI)
      nes.registers.flags.interrupts_disabled = true;
    },
    7 => {
      // Read PCH from interrupt vector
      let interrupt_vector = nes.cpu.temp_address;
      nes.registers.pc = (nes.registers.pc & 0x00FF) | ((read_byte(nes, interrupt_vector + 1) as u16) << 8);
      // All done!
      nes.cpu.tick = 0;
      nes.cpu.service_routine_active = false;
    },
    _ => ()
  }
}

pub fn brk(nes: &mut NesState) {
  // BRK's first cycle is the same as any ordinary instruction.
  match nes.cpu.tick {
    2 => {
      let pc = nes.registers.pc;
      let _ = read_byte(nes, pc);
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
    },
    3 ... 4 => service_interrupt(nes),
    5 => {
      // At this point, NMI always takes priority, otherwise we run
      // an IRQ. This is the source of the BRK hijack quirk / bug.
      if nes.cpu.nmi_requested {
        nes.cpu.nmi_requested = false;
        nes.cpu.temp_address = 0xFFFA;
      } else {
        nes.cpu.temp_address = 0xFFFE;
      }
      // Here we set the B flag to signal a BRK, even if we end up servicing an NMI instead.
      let status_byte = cpu::status_as_byte(&mut nes.registers, true);
      cpu::push(nes, status_byte);
    },
    6 ... 7 => service_interrupt(nes),
    _ => ()
  }
}

// Memory Utilities
pub fn push(nes: &mut NesState, data: u8) {
    let address = (nes.registers.s as u16) + 0x0100;
    write_byte(nes, address, data);
    nes.registers.s = nes.registers.s.wrapping_sub(1);
}

pub fn pop(nes: &mut NesState) -> u8 {
    nes.registers.s = nes.registers.s.wrapping_add(1);
    let address = (nes.registers.s as u16) + 0x0100;
    return read_byte(nes, address);
}

pub fn jmp_absolute(nes: &mut NesState) {
  match nes.cpu.tick {
    2 => addressing::read_address_low(nes),
    3 => {
      addressing::read_address_high(nes);
      nes.registers.pc = nes.cpu.temp_address;
      nes.cpu.tick = 0;
    },
    _ => ()
  }
}

pub fn jmp_indirect(nes: &mut NesState) {
  match nes.cpu.tick {
    2 => addressing::read_address_low(nes),
    3 => addressing::read_address_high(nes),
    4 => {
      let temp_address = nes.cpu.temp_address;
      nes.cpu.data1 = read_byte(nes, temp_address);
    },
    5 => {
      // Add 1 to temp address's low byte only (don't cross page boundary)
      let mut temp_address = nes.cpu.temp_address;
      temp_address = (temp_address & 0xFF00) | ((temp_address + 1) & 0x00FF);
      nes.cpu.data2 = read_byte(nes, temp_address);
      // Set PC to the combined address and exit
      nes.registers.pc = ((nes.cpu.data2 as u16) << 8) | (nes.cpu.data1 as u16);
      nes.cpu.tick = 0;
    },

    _ => ()
  }
}

pub fn jsr(nes: &mut NesState) {
  match nes.cpu.tick {
    2 => addressing::read_address_low(nes),
    3 => {/* Internal Operation */},
    4 => {
      let pch = ((nes.registers.pc & 0xFF00) >> 8) as u8;
      push(nes, pch);
    },
    5 => {
      let pcl = (nes.registers.pc & 0x00FF) as u8;
      push(nes, pcl);
    },
    6 => {
      addressing::read_address_high(nes);
      nes.registers.pc = nes.cpu.temp_address;
      nes.cpu.tick = 0;
    },
    _ => ()
  };
}

pub fn rti(nes: &mut NesState) {
  match nes.cpu.tick {
    2 => addressing::dummy_data1(nes),
    3 => {/* Would incremeent S here */},
    4 => {
      let s = pop(nes);
      cpu::set_status_from_byte(&mut nes.registers, s);
    },
    5 => {
      // Read PCL
      nes.cpu.data1 = pop(nes);
    },
    6 => {
      // Read PCH
      let pch = pop(nes) as u16;
      let pcl = nes.cpu.data1 as u16;
      nes.registers.pc = (pch << 8) | pcl;
      nes.cpu.tick = 0;
    },
    _ => ()
  };
}

pub fn rts(nes: &mut NesState) {
  match nes.cpu.tick {
    2 => addressing::dummy_data1(nes),
    3 => {/* Would incremeent S here */},
    4 => {
      // Read PCL
      nes.cpu.data1 = pop(nes);
    },
    5 => {
      let pch = pop(nes) as u16;
      let pcl = nes.cpu.data1 as u16;
      nes.registers.pc = (pch << 8) | pcl;
    },
    6 => {
      nes.registers.pc = nes.registers.pc.wrapping_add(0x1);
      nes.cpu.tick = 0;
    },
    _ => ()
  };
}

// Opcodes which access the stack
pub fn pha(nes: &mut NesState) {
  match nes.cpu.tick {
    2 => addressing::dummy_data1(nes),
    3 => {
      let a = nes.registers.a;
      push(nes, a);
      nes.cpu.tick = 0;
    },
    _ => (),
  }
}

pub fn php(nes: &mut NesState) {
  match nes.cpu.tick {
    2 => addressing::dummy_data1(nes),
    3 => {
      let status = cpu::status_as_byte(&mut nes.registers, true);
      push(nes, status);
      nes.cpu.tick = 0;
    },
    _ => (),
  }
}

pub fn pla(nes: &mut NesState) {
  match nes.cpu.tick {
    2 => addressing::dummy_data1(nes),
    3 => {/* Increment S */},
    4 => {
      let a = pop(nes);
      nes.registers.a = a;
      nes.registers.flags.zero = nes.registers.a == 0;
      nes.registers.flags.negative = nes.registers.a & 0x80 != 0;
      nes.cpu.tick = 0;
    },
    _ => (),
  }
}

pub fn plp(nes: &mut NesState) {
  match nes.cpu.tick {
    2 => addressing::dummy_data1(nes),
    3 => {/* Increment S */},
    4 => {
      let s = pop(nes);
      cpu::set_status_from_byte(&mut nes.registers, s);
      nes.cpu.tick = 0;
    },
    _ => (),
  }
}