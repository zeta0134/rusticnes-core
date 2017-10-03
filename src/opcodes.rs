use cpu;
use cpu::Registers;

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