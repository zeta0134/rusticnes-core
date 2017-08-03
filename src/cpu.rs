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

// Memory Utilities
fn push(registers: &mut Registers, memory: &mut [u8], data: u8) {
    memory[registers.s as usize] = data;
    registers.s = registers.s.wrapping_sub(1);
}

fn pop(registers: &mut Registers, memory: &[u8]) -> u8 {
    registers.s = registers.s.wrapping_add(1);
    return memory[registers.s as usize];
}

fn status_as_byte(registers: &mut Registers, s_flag: bool) -> u8 {
    return (registers.flags.carry     as u8 +
            (registers.flags.zero      as u8) << 1 +
            (registers.flags.interrupt as u8) << 2 +
            (registers.flags.decimal   as u8) << 3 +
            (s_flag                    as u8) << 4 +
            (1u8                            ) << 5 + // always set
            (registers.flags.overflow  as u8) << 6 +
            (registers.flags.negative  as u8) << 7)
}

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

// Branch if Overflow Clear
fn bvc(registers: &mut Registers, offset: u8) {
    if (!(registers.flags.overflow)) {
        registers.pc = registers.pc.wrapping_add(offset as u16);
    }
}

// Branch if Overflow Set
fn bvs(registers: &mut Registers, offset: u8) {
    if (registers.flags.overflow) {
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

fn brk(registers: &mut Registers, memory: &mut [u8]) {
    // Push PC and processor status to stack
    let pc_high = (registers.pc & 0xFF00 >> 8) as u8;
    let pc_low =  (registers.pc & 0x00FF) as u8;
    push(registers, memory, pc_high);
    push(registers, memory, pc_low);
    let status_byte = status_as_byte(registers, true);
    push(registers, memory, status_byte);
    // Set PC to interrupt vector at FFFE/FFFF
    registers.pc = memory[0xFFFE] as u16 + ((memory[0xFFFF] as u16) << 8);
}




// ADDRESSING
