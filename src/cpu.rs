struct Flags {
    carry: bool,
    zero: bool,
    interrupt: bool,
    decimal: bool,
    overflow: bool,
    negative: bool,
}

struct Registers {
    a: u8,
    x: u8,
    y: u8,
    pc: u16,
    s: u8,
    flags: Flags,
}

// Initial reference implementation based on http://obelisk.me.uk/6502/reference.html

// OPCODES
fn overflow(a: u8, b: u8, result: u8) -> bool {
    return (((!(a ^ b)) & (a ^ result)) & 0x80) != 0
}

// Add with Carry
fn adc(registers: &mut Registers, data: u8) {
    let result: u16 = registers.a as u16 + data as u16 + registers.flags.carry as u16;
    registers.flags.carry = result > 0xFF;
    registers.flags.overflow = overflow(registers.a, data, result as u8);
    registers.a = (result & 0xFF) as u8;
    registers.flags.zero = registers.a == 0;
    registers.flags.negative = registers.a & 0x80 != 0;
}

fn and(registers: &mut Registers, data: u8) {
    registers.a = registers.a & data;
    registers.flags.zero = registers.a == 0;
    registers.flags.negative = registers.a & 0x80 != 0;
}

fn asl(registers: &mut Registers, data: u8) -> u8 {
    registers.flags.carry = data & 0x80 != 0;
    registers.a = registers.a << 1;
    registers.flags.zero = data == 0;
    registers.flags.negative = data & 0x80 != 0;
    return data;
}

// Branch if Carry Clear
fn bcc(registers: &mut Registers, offset: u8) {
    if (!(registers.flags.carry)) {
        registers.pc = registers.pc.wrapping_add(offset as u16);
    }
}

// Branch if Carry Set
fn bcs(registers: &mut Registers, offset: u8) {
    if (registers.flags.carry) {
        registers.pc = registers.pc.wrapping_add(offset as u16);
    }
}

// Branch if Equal
fn beq(registers: &mut Registers, offset: u8) {
    if (registers.flags.zero) {
        registers.pc = registers.pc.wrapping_add(offset as u16);
    }
}

// Branch if Not Equal
fn bne(registers: &mut Registers, offset: u8) {
    if (!(registers.flags.zero)) {
        registers.pc = registers.pc.wrapping_add(offset as u16);
    }
}

// Branch if Negative
fn bmi(registers: &mut Registers, offset: u8) {
    if (registers.flags.negative) {
        registers.pc = registers.pc.wrapping_add(offset as u16);
    }
}

// Branch if Positive
fn bpl(registers: &mut Registers, offset: u8) {
    if (!(registers.flags.negative)) {
        registers.pc = registers.pc.wrapping_add(offset as u16);
    }
}

// Bit Test
fn bit(registers: &mut Registers, data: u8) {
    let result: u8 = registers.a & data;
    registers.flags.zero = result == 0;
    registers.flags.overflow = result & 0x40 != 0;
    registers.flags.negative = result & 0x80 != 0;
}

// Addressing Modes
