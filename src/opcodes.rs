use cpu;
use cpu::Registers;
use nes::NesState;
use memory::read_byte;

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

// Branch
pub fn branch(nes: &mut NesState) {
  // Note: the documentation for branch timing, located at http://nesdev.com/6502_cpu.txt includes
  // the first cycle of the next instruction. Thus, when bailing here, we set nes.cpu.tick to 1
  // instead of zero, so we don't execute the fetch step a second time when we're done.
  match nes.cpu.tick {
    2 => {
      // data1 is already filled
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

      if nes.registers.pc & 0xFF != ((nes.cpu.data2 as u16) << 8) {
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