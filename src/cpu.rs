use memory::CpuMemory;

pub struct Flags {
    pub carry: bool,
    pub zero: bool,
    pub decimal: bool,
    pub interrupts_disabled: bool,
    pub overflow: bool,
    pub negative: bool,
}

pub struct Registers {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub pc: u16,
    pub s: u8,
    pub flags: Flags,
}

// Initial reference implementation based on http://obelisk.me.uk/6502/reference.html

// Memory Utilities
fn push(registers: &mut Registers, memory: &mut CpuMemory, data: u8) {
    memory[registers.s as u16] = data;
    registers.s = registers.s.wrapping_sub(1);
}

fn pop(registers: &mut Registers, memory: &CpuMemory) -> u8 {
    registers.s = registers.s.wrapping_add(1);
    return memory[registers.s as u16];
}

fn status_as_byte(registers: &mut Registers, s_flag: bool) -> u8 {
    return registers.flags.carry     as u8 +
           (registers.flags.zero      as u8) << 1 +
           (registers.flags.interrupts_disabled as u8) << 2 +
           (registers.flags.decimal   as u8) << 3 +
           (s_flag                    as u8) << 4 +
           (1u8                            ) << 5 + // always set
           (registers.flags.overflow  as u8) << 6 +
           (registers.flags.negative  as u8) << 7
}

fn set_status_from_byte(registers: &mut Registers, data: u8) {
    registers.flags.carry =    data & (1 << 0) != 0;
    registers.flags.zero =     data & (1 << 1) != 0;
    registers.flags.interrupts_disabled = data & (1 << 2) != 0;
    registers.flags.decimal =  data & (1 << 3) != 0;
    // bits 4 and 5, the s_flag, do not actually exist
    registers.flags.overflow = data & (1 << 6) != 0;
    registers.flags.negative = data & (1 << 7) != 0;
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
fn bcc(registers: &mut Registers, offset: i8) {
    if !(registers.flags.carry) {
        registers.pc = registers.pc.wrapping_add(offset as u16);
    }
}

// Branch if Carry Set
fn bcs(registers: &mut Registers, offset: i8) {
    if registers.flags.carry {
        registers.pc = registers.pc.wrapping_add(offset as u16);
    }
}

// Branch if Equal
fn beq(registers: &mut Registers, offset: i8) {
    if registers.flags.zero {
        registers.pc = registers.pc.wrapping_add(offset as u16);
    }
}

// Branch if Not Equal
fn bne(registers: &mut Registers, offset: i8) {
    if !(registers.flags.zero) {
        registers.pc = registers.pc.wrapping_add(offset as u16);
    }
}

// Branch if Negative
fn bmi(registers: &mut Registers, offset: i8) {
    if registers.flags.negative {
        registers.pc = registers.pc.wrapping_add(offset as u16);
    }
}

// Branch if Positive
fn bpl(registers: &mut Registers, offset: i8) {
    if !(registers.flags.negative) {
        registers.pc = registers.pc.wrapping_add(offset as u16);
    }
}

// Branch if Overflow Clear
fn bvc(registers: &mut Registers, offset: i8) {
    if !(registers.flags.overflow) {
        registers.pc = registers.pc.wrapping_add(offset as u16);
    }
}

// Branch if Overflow Set
fn bvs(registers: &mut Registers, offset: i8) {
    if registers.flags.overflow {
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

fn brk(registers: &mut Registers, memory: &mut CpuMemory) {
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

// Clear carry flag
fn clc(registers: &mut Registers) {
    registers.flags.carry = false
}

// Clear decimal flag
fn cld(registers: &mut Registers) {
    registers.flags.decimal = false
}

// Clear interrupt disable (enbales interrupts?)
fn cli(registers: &mut Registers) {
    registers.flags.interrupts_disabled = false
}

// Clear overflow flag
fn clv(registers: &mut Registers) {
    registers.flags.overflow = false
}

// Compare Accumulator
fn cmp(registers: &mut Registers, data: u8) {
    registers.flags.carry = registers.a >= data;
    let result: u8 = registers.a.wrapping_sub(data);
    registers.flags.zero = result == 0;
    registers.flags.negative = result & 0x80 != 0;
}

// Compare X
fn cpx(registers: &mut Registers, data: u8) {
    registers.flags.carry = registers.x >= data;
    let result: u8 = registers.x.wrapping_sub(data);
    registers.flags.zero = result == 0;
    registers.flags.negative = result & 0x80 != 0;
}

// Compare Y
fn cpy(registers: &mut Registers, data: u8) {
    registers.flags.carry = registers.y >= data;
    let result: u8 = registers.y.wrapping_sub(data);
    registers.flags.zero = result == 0;
    registers.flags.negative = result & 0x80 != 0;
}

// Decrement Memory
fn dec(registers: &mut Registers, data: u8) -> u8 {
    let result: u8 = data.wrapping_sub(1);
    registers.flags.zero = result == 0;
    registers.flags.negative = result & 0x80 != 0;
    return result;
}

// Decrement X
fn dex(registers: &mut Registers) {
    registers.x = registers.x.wrapping_sub(1);
    registers.flags.zero = registers.x == 0;
    registers.flags.negative = registers.x & 0x80 != 0;
}

// Decrement Y
fn dey(registers: &mut Registers) {
    registers.y = registers.y.wrapping_sub(1);
    registers.flags.zero = registers.y == 0;
    registers.flags.negative = registers.y & 0x80 != 0;
}

// Exclusive OR
fn eor(registers: &mut Registers, data: u8) {
    registers.a = registers.a | data;
    registers.flags.zero = registers.a == 0;
    registers.flags.negative = registers.a & 0x80 != 0;
}

// Increment Memory
fn inc(registers: &mut Registers, data: u8) -> u8 {
    let result: u8 = data.wrapping_add(1);
    registers.flags.zero = result == 0;
    registers.flags.negative = result & 0x80 != 0;
    return result;
}

// Increment X
fn inx(registers: &mut Registers) {
    registers.x = registers.x.wrapping_add(1);
    registers.flags.zero = registers.x == 0;
    registers.flags.negative = registers.x & 0x80 != 0;
}

// Decrement Y
fn iny(registers: &mut Registers) {
    registers.y = registers.y.wrapping_add(1);
    registers.flags.zero = registers.y == 0;
    registers.flags.negative = registers.y & 0x80 != 0;
}

// Jump
fn jmp(registers: &mut Registers, address: u16) {
    registers.pc = address;
}

// Jump to Subroutine
fn jsr(registers: &mut Registers, memory: &mut CpuMemory, address: u16) {
    let return_address = registers.pc.wrapping_sub(1);
    let addr_high = (return_address & 0xFF00 >> 8) as u8;
    let addr_low =  (return_address & 0x00FF) as u8;
    push(registers, memory, addr_high);
    push(registers, memory, addr_low);
    registers.pc = address;
}

// Load Accumulator
fn lda(registers: &mut Registers, data: u8) {
    registers.a = data;
    registers.flags.zero = registers.a == 0;
    registers.flags.negative = registers.a & 0x80 != 0;
}

// Load X
fn ldx(registers: &mut Registers, data: u8) {
    registers.x = data;
    registers.flags.zero = registers.x == 0;
    registers.flags.negative = registers.x & 0x80 != 0;
}

// Load Y
fn ldy(registers: &mut Registers, data: u8) {
    registers.y = data;
    registers.flags.zero = registers.y == 0;
    registers.flags.negative = registers.y & 0x80 != 0;
}

// Logical shift right
fn lsr(registers: &mut Registers, data: u8) -> u8 {
    registers.flags.carry = data & 0x1 != 0;
    let result: u8 = data >> 1;
    registers.flags.zero = result == 0;
    return result;
}

// No operation!
fn nop() {
}

// Logical inclusive OR
fn ora(registers: &mut Registers, data: u8) {
    registers.a = registers.a | data;
    registers.flags.zero = registers.a == 0;
    registers.flags.negative = registers.a & 0x80 != 0;
}

// Push Accumulator
fn pha(registers: &mut Registers, memory: &mut CpuMemory) {
    let a = registers.a;
    push(registers, memory, a);
}

// Push Processor Status
fn php(registers: &mut Registers, memory: &mut CpuMemory) {
    let processor_status = status_as_byte(registers, false);
    push(registers, memory, processor_status);
}

// Pull Accumulator
fn pla(registers: &mut Registers, memory: &mut CpuMemory) {
    registers.a = pop(registers, memory);
}

// Pull Procesor Status
fn plp(registers: &mut Registers, memory: &mut CpuMemory) {
    let processor_status = pop(registers, memory);
    set_status_from_byte(registers, processor_status);
}

// Rotate left
fn rol(registers: &mut Registers, data: u8) -> u8 {
    let old_carry = registers.flags.carry;
    registers.flags.carry = data & 0x80 != 0;
    let result = (data << 1) + old_carry as u8;
    registers.flags.zero = result == 0;
    registers.flags.negative = result & 0x80 != 0;
    return result;
}

// Rotate Right
fn ror(registers: &mut Registers, data: u8) -> u8 {
    let old_carry = registers.flags.carry;
    registers.flags.carry = data & 0x01 != 0;
    let result = (data >> 1) + ((old_carry as u8) << 7);
    registers.flags.zero = result == 0;
    registers.flags.negative = result & 0x80 != 0;
    return result;
}

// Return from Interrupt
fn rti(registers: &mut Registers, memory: &mut CpuMemory) {
    let status_byte = pop(registers, memory);
    set_status_from_byte(registers, status_byte);
    let pc_low = pop(registers, memory) as u16;
    let pc_high = pop(registers, memory) as u16;
    let pc = (pc_high << 8) + pc_low;
    registers.pc = pc;
}

// Return from Subroutine
fn rts(registers: &mut Registers, memory: &mut CpuMemory) {
    let pc_low = pop(registers, memory) as u16;
    let pc_high = pop(registers, memory) as u16;
    let pc = (pc_high << 8) + pc_low;
    registers.pc = pc.wrapping_add(1);
}

// Subtract with Carry
fn sbc(registers: &mut Registers, data: u8) {
    // Preload the carry into bit 8
    let inverted_data = data ^ 0xFF;
    adc(registers, data);
}

// Set Carry Flag
fn sec(registers: &mut Registers) {
    registers.flags.carry = true;
}

// Set Decimal Flag
fn sed(registers: &mut Registers) {
    registers.flags.decimal = true;
}

// Set Interrupt Disable Flag
fn sei(registers: &mut Registers) {
    registers.flags.interrupts_disabled = true;
}

// Store Accumulator
fn sta(registers: &mut Registers) -> u8 {
    return registers.a
}

// Store X
fn stx(registers: &mut Registers) -> u8 {
    return registers.x
}

// Store Y
fn sty(registers: &mut Registers) -> u8 {
    return registers.y
}

// Transfer A -> X
fn tax(registers: &mut Registers) {
    registers.x = registers.a;
    registers.flags.zero = registers.x == 0;
    registers.flags.negative = registers.x & 0x80 != 0;
}

// Transfer A -> Y
fn tay(registers: &mut Registers) {
    registers.y = registers.a;
    registers.flags.zero = registers.y == 0;
    registers.flags.negative = registers.y & 0x80 != 0;
}

// Transfer S -> X
fn tsx(registers: &mut Registers) {
    registers.x = registers.s;
    registers.flags.zero = registers.x == 0;
    registers.flags.negative = registers.x & 0x80 != 0;
}

// Transfer X -> A
fn txa(registers: &mut Registers) {
    registers.a = registers.x;
    registers.flags.zero = registers.a == 0;
    registers.flags.negative = registers.a & 0x80 != 0;
}

// Transfer X -> S
fn txs(registers: &mut Registers) {
    registers.s = registers.x;
}

// Transfer Y -> A
fn tya(registers: &mut Registers) {
    registers.a = registers.y;
    registers.flags.zero = registers.a == 0;
    registers.flags.negative = registers.a & 0x80 != 0;
}

fn undefined() {
    println!("Undefined opcode!");
}

// Addressing Modes
fn immediate(registers: &mut Registers) -> u16 {
    let address = registers.pc;
    registers.pc = registers.pc.wrapping_add(1);
    return address as u16;
}

fn zero_page(registers: &mut Registers, memory: &mut CpuMemory) -> u16 {
    let offset = memory[registers.pc];
    registers.pc = registers.pc.wrapping_add(1);
    return offset as u16;
}

fn zero_x(registers: &mut Registers, memory: &mut  CpuMemory) -> u16 {
    let offset = memory[registers.pc].wrapping_add(registers.x);
    registers.pc = registers.pc.wrapping_add(1);
    return offset as u16;
}

fn zero_y(registers: &mut Registers, memory: &mut CpuMemory) -> u16 {
    let offset = memory[registers.pc].wrapping_add(registers.y);
    registers.pc = registers.pc.wrapping_add(1);
    return offset as u16;
}

fn absolute(registers: &mut Registers, memory: &mut CpuMemory) -> u16 {
    let address_low = memory[registers.pc];
    registers.pc = registers.pc.wrapping_add(1);
    let address_high = memory[registers.pc];
    registers.pc = registers.pc.wrapping_add(1);
    let address = ((address_high as u16) << 8) + (address_low as u16);
    return address as u16;
}

fn absolute_x(registers: &mut Registers, memory: &mut  CpuMemory) -> u16 {
    let address_low = memory[registers.pc];
    registers.pc = registers.pc.wrapping_add(1);
    let address_high = memory[registers.pc];
    registers.pc = registers.pc.wrapping_add(1);
    let mut address = ((address_high as u16) << 8) + (address_low as u16);
    address = address.wrapping_add(registers.x as u16);
    return address as u16;
}

fn absolute_y(registers: &mut Registers, memory: &mut  CpuMemory) -> u16 {
    let address_low = memory[registers.pc];
    registers.pc = registers.pc.wrapping_add(1);
    let address_high = memory[registers.pc];
    registers.pc = registers.pc.wrapping_add(1);
    let mut address = ((address_high as u16) << 8) + (address_low as u16);
    address = address.wrapping_add(registers.y as u16);
    return address as u16;
}

// Only used by jump
fn indirect(registers: &mut Registers, memory: &mut CpuMemory) -> u16 {
    let indirect_low = memory[registers.pc];
    registers.pc = registers.pc.wrapping_add(1);
    let indirect_high = memory[registers.pc];
    registers.pc = registers.pc.wrapping_add(1);
    let mut indirect_address = ((indirect_high as u16) << 8) + (indirect_low as u16);

    let address_low = memory[indirect_address];
    indirect_address = indirect_address.wrapping_add(1);
    let address_high = memory[indirect_address];
    let address = ((address_high as u16) << 8) + (address_low as u16);

    return address as u16;
}

fn indexed_indirect_x(registers: &mut Registers, memory: &mut  CpuMemory) -> u16 {
    let mut table_address = memory[registers.pc as u16].wrapping_add(registers.x);
    registers.pc = registers.pc.wrapping_add(1);
    let address_low = memory[table_address as u16];
    table_address = table_address.wrapping_add(1);
    let address_high = memory[table_address as u16];
    let address = ((address_high as u16) << 8) + (address_low as u16);
    return address as u16;
}

fn indirect_indexed_y(registers: &mut Registers, memory: &mut CpuMemory) -> u16 {
    let mut offset = memory[registers.pc as u16];
    registers.pc = registers.pc.wrapping_add(1);
    let address_low = memory[offset as u16];
    offset = offset.wrapping_add(1);
    let address_high = memory[offset as u16];
    let mut address = ((address_high as u16) << 8) + (address_low as u16);
    address = address.wrapping_add(registers.y as u16);
    return address as u16;
}

pub fn process_instruction(registers: &mut Registers, memory: &mut CpuMemory) {
    let opcode = memory[registers.pc];
    registers.pc = registers.pc.wrapping_add(1);

    match opcode {
        // Add with Carry
        0x69 => { let address = immediate(registers);
                  adc(registers, memory[address])
        },
        0x65 => { let address = zero_page(registers, memory);
                  adc(registers, memory[address])
        },
        0x75 => { let address = zero_x(registers, memory);
                  adc(registers, memory[address])
        },
        0x6D => { let address = absolute(registers, memory);
                  adc(registers, memory[address])
        },
        0x7D => { let address = absolute_x(registers, memory);
                  adc(registers, memory[address])
        },
        0x79 => { let address = absolute_y(registers, memory);
                  adc(registers, memory[address])
        },
        0x61 => { let address = indexed_indirect_x(registers, memory);
                  adc(registers, memory[address])
        },
        0x71 => { let address = indirect_indexed_y(registers, memory);
                  adc(registers, memory[address])
        },

        // Logical AND
        0x29 => { let address = immediate(registers);
                  and(registers, memory[address])
        },
        0x25 => { let address = zero_page(registers, memory);
                  and(registers, memory[address])
        },
        0x35 => { let address = zero_x(registers, memory);
                  and(registers, memory[address])
        },
        0x2D => { let address = absolute(registers, memory);
                  and(registers, memory[address])
        },
        0x3D => { let address = absolute_x(registers, memory);
                  and(registers, memory[address])
        },
        0x39 => { let address = absolute_y(registers, memory);
                  and(registers, memory[address])
        },
        0x21 => { let address = indexed_indirect_x(registers, memory);
                  and(registers, memory[address])
        },
        0x31 => { let address = indirect_indexed_y(registers, memory);
                  and(registers, memory[address])
        },

        // Arithmetic Shift Left
        0x0A => { let value = registers.a;
                  registers.a = asl(registers, value)
        },
        0x06 => { let address = zero_page(registers, memory);
                  memory[address] = asl(registers, memory[address])
        },
        0x16 => { let address = zero_x(registers, memory);
                  memory[address] = asl(registers, memory[address])
        },
        0x0E => { let address = absolute(registers, memory);
                  memory[address] = asl(registers, memory[address])
        },
        0x1E => { let address = absolute_x(registers, memory);
                  memory[address] = asl(registers, memory[address])
        },

        // Branching
        0x90 => { let address = immediate(registers);
                  bcc(registers, memory[address] as i8)
        },
        0xB0 => { let address = immediate(registers);
                  bcs(registers, memory[address] as i8)
        },
        0xF0 => { let address = immediate(registers);
                  beq(registers, memory[address] as i8)
        },
        0x30 => { let address = immediate(registers);
                  bmi(registers, memory[address] as i8)
        },
        0xD0 => { let address = immediate(registers);
                  bne(registers, memory[address] as i8)
        },
        0x10 => { let address = immediate(registers);
                  bpl(registers, memory[address] as i8)
        },
        0x50 => { let address = immediate(registers);
                  bvc(registers, memory[address] as i8)
        },
        0x70 => { let address = immediate(registers);
                  bvs(registers, memory[address] as i8)
        },

        // Bit Test
        0x24 => { let address = zero_page(registers, memory);
                  bit(registers, memory[address])
        },
        0x2C => { let address = absolute(registers, memory);
                  bit(registers, memory[address])
        },

        // Break - Force Interrupt
        0x00 => brk(registers, memory),

        // Clear Flags
        0x18 => clc(registers),
        0xD8 => cld(registers),
        0x58 => cli(registers),
        0xB8 => clv(registers),

        // Compare
        0xC9 => { let address = immediate(registers);
                  cmp(registers, memory[address])
        },
        0xC5 => { let address = zero_page(registers, memory);
                  cmp(registers, memory[address])
        },
        0xD5 => { let address = zero_x(registers, memory);
                  cmp(registers, memory[address])
        },
        0xCD => { let address = absolute(registers, memory);
                  cmp(registers, memory[address])
        },
        0xDD => { let address = absolute_x(registers, memory);
                  cmp(registers, memory[address])
        },
        0xD9 => { let address = absolute_y(registers, memory);
                  cmp(registers, memory[address])
        },
        0xC1 => { let address = indexed_indirect_x(registers, memory);
                  cmp(registers, memory[address])
        },
        0xD1 => { let address = indirect_indexed_y(registers, memory);
                  cmp(registers, memory[address])
        },

        // Compare X
        0xE0 => { let address = immediate(registers);
                  cpx(registers, memory[address])
        },
        0xE4 => { let address = zero_page(registers, memory);
                  cpx(registers, memory[address])
        },
        0xEC => { let address = absolute(registers, memory);
                  cpx(registers, memory[address])
        },

        // Compare Y
        0xC0 => { let address = immediate(registers);
                  cpy(registers, memory[address])
        },
        0xC4 => { let address = zero_page(registers, memory);
                  cpy(registers, memory[address])
        },
        0xCC => { let address = absolute(registers, memory);
                  cpy(registers, memory[address])
        },

        // Decrement
        0xC6 => { let address = zero_page(registers, memory);
                  memory[address] = dec(registers, memory[address])
        },
        0xD6 => { let address = zero_x(registers, memory);
                  memory[address] = dec(registers, memory[address])
        },
        0xCE => { let address = absolute(registers, memory);
                  memory[address] = dec(registers, memory[address])
        },
        0xDE => { let address = absolute_x(registers, memory);
                  memory[address] = dec(registers, memory[address])
        },
        0xCA => dex(registers),
        0x88 => dey(registers),

        // Logical Exclusive OR
        0x49 => { let address = immediate(registers);
                  eor(registers, memory[address])
        },
        0x45 => { let address = zero_page(registers, memory);
                  eor(registers, memory[address])
        },
        0x55 => { let address = zero_x(registers, memory);
                  eor(registers, memory[address])
        },
        0x4D => { let address = absolute(registers, memory);
                  eor(registers, memory[address])
        },
        0x5D => { let address = absolute_x(registers, memory);
                  eor(registers, memory[address])
        },
        0x59 => { let address = absolute_y(registers, memory);
                  eor(registers, memory[address])
        },
        0x41 => { let address = indexed_indirect_x(registers, memory);
                  eor(registers, memory[address])
        },
        0x51 => { let address = indirect_indexed_y(registers, memory);
                  eor(registers, memory[address])
        },

        // Increment
        0xE6 => { let address = zero_page(registers, memory);
                  memory[address] = inc(registers, memory[address])
        },
        0xF6 => { let address = zero_x(registers, memory);
                  memory[address] = inc(registers, memory[address])
        },
        0xEE => { let address = absolute(registers, memory);
                  memory[address] = inc(registers, memory[address])
        },
        0xFE => { let address = absolute_x(registers, memory);
                  memory[address] = inc(registers, memory[address])
        },
        0xE8 => inx(registers),
        0xC8 => iny(registers),

        // Jump
        0x4C => { let address = absolute(registers, memory);
                  jmp(registers, address as u16)
        },
        0x6C => { let address = indirect(registers, memory);
                  jmp(registers, address as u16)
        },

        // Jump to Subroutine
        0x20 => { let address = absolute(registers, memory);
                  jsr(registers, memory, address as u16)
        },

        // Load Accumulator
        0xA9 => { let address = immediate(registers);
                  lda(registers, memory[address])
        },
        0xA5 => { let address = zero_page(registers, memory);
                  lda(registers, memory[address])
        },
        0xB5 => { let address = zero_x(registers, memory);
                  lda(registers, memory[address])
        },
        0xAD => { let address = absolute(registers, memory);
                  lda(registers, memory[address])
        },
        0xBD => { let address = absolute_x(registers, memory);
                  lda(registers, memory[address])
        },
        0xB9 => { let address = absolute_y(registers, memory);
                  lda(registers, memory[address])
        },
        0xA1 => { let address = indexed_indirect_x(registers, memory);
                  lda(registers, memory[address])
        },
        0xB1 => { let address = indirect_indexed_y(registers, memory);
                  lda(registers, memory[address])
        },

        // Load X
        0xA2 => { let address = immediate(registers);
                  ldx(registers, memory[address])
        },
        0xA6 => { let address = zero_page(registers, memory);
                  ldx(registers, memory[address])
        },
        0xB6 => { let address = zero_y(registers, memory);
                  ldx(registers, memory[address])
        },
        0xAE => { let address = absolute(registers, memory);
                  ldx(registers, memory[address])
        },
        0xBE => { let address = absolute_y(registers, memory);
                  ldx(registers, memory[address])
        },

        // Load Y
        0xA0 => { let address = immediate(registers);
                  ldy(registers, memory[address])
        },
        0xA4 => { let address = zero_page(registers, memory);
                  ldy(registers, memory[address])
        },
        0xB4 => { let address = zero_x(registers, memory);
                  ldy(registers, memory[address])
        },
        0xAC => { let address = absolute(registers, memory);
                  ldy(registers, memory[address])
        },
        0xBC => { let address = absolute_x(registers, memory);
                  ldy(registers, memory[address])
        },

        // Logical Shift Right
        0x4A => { let value = registers.a;
                  registers.a = lsr(registers, value)
        },
        0x46 => { let address = zero_page(registers, memory);
                  memory[address] = lsr(registers, memory[address])
        },
        0x56 => { let address = zero_x(registers, memory);
                  memory[address] = lsr(registers, memory[address])
        },
        0x4E => { let address = absolute(registers, memory);
                  memory[address] = lsr(registers, memory[address])
        },
        0x5E => { let address = absolute_x(registers, memory);
                  memory[address] = lsr(registers, memory[address])
        },

        // NOP!
        0xEA => nop(),

        // Logical Inclusive OR
        0x09 => { let address = immediate(registers);
                  ora(registers, memory[address])
        },
        0x05 => { let address = zero_page(registers, memory);
                  ora(registers, memory[address])
        },
        0x15 => { let address = zero_x(registers, memory);
                  ora(registers, memory[address])
        },
        0x0D => { let address = absolute(registers, memory);
                  ora(registers, memory[address])
        },
        0x1D => { let address = absolute_x(registers, memory);
                  ora(registers, memory[address])
        },
        0x19 => { let address = absolute_y(registers, memory);
                  ora(registers, memory[address])
        },
        0x01 => { let address = indexed_indirect_x(registers, memory);
                  ora(registers, memory[address])
        },
        0x11 => { let address = indirect_indexed_y(registers, memory);
                  ora(registers, memory[address])
        },

        // Push and Pop
        0x48 => pha(registers, memory),
        0x08 => php(registers, memory),
        0x68 => pla(registers, memory),
        0x28 => plp(registers, memory),

        // Rotate Left
        0x2A => { let value = registers.a;
                  registers.a = rol(registers, value)
        },
        0x26 => { let address = zero_page(registers, memory);
                  memory[address] = rol(registers, memory[address])
        },
        0x36 => { let address = zero_x(registers, memory);
                  memory[address] = rol(registers, memory[address])
        },
        0x2E => { let address = absolute(registers, memory);
                  memory[address] = rol(registers, memory[address])
        },
        0x3E => { let address = absolute_x(registers, memory);
                  memory[address] = rol(registers, memory[address])
        },

        // Rotate Right
        0x6A => { let value = registers.a;
                  registers.a = ror(registers, value)
        },
        0x66 => { let address = zero_page(registers, memory);
                  memory[address] = ror(registers, memory[address])
        },
        0x76 => { let address = zero_x(registers, memory);
                  memory[address] = ror(registers, memory[address])
        },
        0x6E => { let address = absolute(registers, memory);
                  memory[address] = ror(registers, memory[address])
        },
        0x7E => { let address = absolute_x(registers, memory);
                  memory[address] = ror(registers, memory[address])
        },

        // Returns
        0x40 => rti(registers, memory),
        0x60 => rts(registers, memory),

        // Subtract with Carry
        0xE9 => { let address = immediate(registers);
                  sbc(registers, memory[address])
        },
        0xE5 => { let address = zero_page(registers, memory);
                  sbc(registers, memory[address])
        },
        0xF5 => { let address = zero_x(registers, memory);
                  sbc(registers, memory[address])
        },
        0xED => { let address = absolute(registers, memory);
                  sbc(registers, memory[address])
        },
        0xFD => { let address = absolute_x(registers, memory);
                  sbc(registers, memory[address])
        },
        0xF9 => { let address = absolute_y(registers, memory);
                  sbc(registers, memory[address])
        },
        0xE1 => { let address = indexed_indirect_x(registers, memory);
                  sbc(registers, memory[address])
        },
        0xF1 => { let address = indirect_indexed_y(registers, memory);
                  sbc(registers, memory[address])
        },

        // Set Flags
        0x38 => sec(registers),
        0xF8 => sed(registers),
        0x78 => sei(registers),

        // Store Accumulator
        0x85 => { let address = zero_page(registers, memory);
                  memory[address] = sta(registers)
        },
        0x95 => { let address = zero_x(registers, memory);
                  memory[address] = sta(registers)
        },
        0x8D => { let address = absolute(registers, memory);
                  memory[address] = sta(registers)
        },
        0x9D => { let address = absolute_x(registers, memory);
                  memory[address] = sta(registers)
        },
        0x99 => { let address = absolute_y(registers, memory);
                  memory[address] = sta(registers)
        },
        0x81 => { let address = indexed_indirect_x(registers, memory);
                  memory[address] = sta(registers)
        },
        0x91 => { let address = indirect_indexed_y(registers, memory);
                  memory[address] = sta(registers)
        },

        // Store X
        0x86 => { let address = zero_page(registers, memory);
                  memory[address] = stx(registers)
        },
        0x96 => { let address = zero_y(registers, memory);
                  memory[address] = stx(registers)
        },
        0x8E => { let address = absolute(registers, memory);
                  memory[address] = stx(registers)
        },

        // Store Y
        0x84 => { let address = zero_page(registers, memory);
                  memory[address] = sty(registers)
        },
        0x94 => { let address = zero_x(registers, memory);
                  memory[address] = sty(registers)
        },
        0x8C => { let address = absolute(registers, memory);
                  memory[address] = sty(registers)
        },

        0xAA => tax(registers),
        0xA8 => tay(registers),
        0xBA => tsx(registers),
        0x8A => txa(registers),
        0x9A => txs(registers),
        0x98 => tya(registers),

        // Undefined Weirdness
        _ => println!("Undefined opcode: {:X}", opcode)
    }
}
