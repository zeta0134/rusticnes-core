use memory::write_byte;
use memory::read_byte;
use nes::NesState;

#[derive(Copy, Clone)]
pub struct Flags {
    pub carry: bool,
    pub zero: bool,
    pub decimal: bool,
    pub interrupts_disabled: bool,
    pub overflow: bool,
    pub negative: bool,

    // internal only
    pub last_nmi: bool,
}

#[derive(Copy, Clone)]
pub struct Registers {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub pc: u16,
    pub s: u8,
    pub flags: Flags,
}

impl Registers {
    pub fn new() -> Registers {
        return Registers {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            s: 0,
            flags: Flags {
                carry: false,
                zero: false,
                interrupts_disabled: false,
                decimal: false,
                overflow: false,
                negative: false,
                last_nmi: false,
            }
        }
    }
}

// Initial reference implementation based on http://obelisk.me.uk/6502/reference.html

// Memory Utilities
fn push(nes: &mut NesState, data: u8) {
    let address = (nes.registers.s as u16) + 0x0100;
    write_byte(nes, address, data);
    nes.registers.s = nes.registers.s.wrapping_sub(1);
}

fn pop(nes: &mut NesState) -> u8 {
    nes.registers.s = nes.registers.s.wrapping_add(1);
    let address = (nes.registers.s as u16) + 0x0100;
    return read_byte(nes, address);
}

fn status_as_byte(registers: &mut Registers, s_flag: bool) -> u8 {
    return (registers.flags.carry     as u8) +
           ((registers.flags.zero      as u8) << 1) +
           ((registers.flags.interrupts_disabled as u8) << 2) +
           ((registers.flags.decimal   as u8) << 3) +
           ((s_flag                    as u8) << 4) +
           ((1u8                            ) << 5) + // always set
           ((registers.flags.overflow  as u8) << 6) +
           ((registers.flags.negative  as u8) << 7)
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
    let result = (data & 0x7F) << 1;
    registers.flags.zero = result == 0;
    registers.flags.negative = result & 0x80 != 0;
    return result;
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
    registers.flags.overflow = data & 0x40 != 0;
    registers.flags.negative = data & 0x80 != 0;
}

fn brk(nes: &mut NesState) {
    // Push PC and processor status to stack
    let return_address = nes.registers.pc.wrapping_add(1);
    let addr_high = ((return_address & 0xFF00) >> 8) as u8;
    let addr_low =  (return_address & 0x00FF) as u8;
    push(nes, addr_high);
    push(nes, addr_low);
    let status_byte = status_as_byte(&mut nes.registers, true);
    push(nes, status_byte);
    // Set PC to interrupt vector at FFFE/FFFF
    nes.registers.pc = read_byte(nes, 0xFFFE) as u16 | ((read_byte(nes, 0xFFFF) as u16) << 8);
    // Disable interrupts
    nes.registers.flags.interrupts_disabled = true;
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
    registers.a = registers.a ^ data;
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
fn jsr(nes: &mut NesState, address: u16) {
    let return_address = nes.registers.pc.wrapping_sub(1);
    let addr_high = ((return_address & 0xFF00) >> 8) as u8;
    let addr_low =  (return_address & 0x00FF) as u8;
    push(nes, addr_high);
    push(nes, addr_low);
    nes.registers.pc = address;
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
    registers.flags.negative = false;
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
fn pha(nes: &mut NesState) {
    let a = nes.registers.a;
    push(nes, a);
}

// Push Processor Status
fn php(nes: &mut NesState) {
    let processor_status = status_as_byte(&mut nes.registers, true);
    push(nes, processor_status);
}

// Pull Accumulator
fn pla(nes: &mut NesState) {
    nes.registers.a = pop(nes);
    nes.registers.flags.zero = nes.registers.a == 0;
    nes.registers.flags.negative = nes.registers.a & 0x80 != 0;
}

// Pull Procesor Status
fn plp(nes: &mut NesState) {
    let processor_status = pop(nes);
    set_status_from_byte(&mut nes.registers, processor_status);
}

// Rotate left
fn rol(registers: &mut Registers, data: u8) -> u8 {
    let old_carry = registers.flags.carry;
    registers.flags.carry = (data & 0x80) != 0;
    let result = (data << 1) | (old_carry as u8);
    registers.flags.zero = result == 0;
    registers.flags.negative = (result & 0x80) != 0;
    return result;
}

// Rotate Right
fn ror(registers: &mut Registers, data: u8) -> u8 {
    let old_carry = registers.flags.carry;
    registers.flags.carry = (data & 0x01) != 0;
    let result = (data >> 1) | ((old_carry as u8) << 7);
    registers.flags.zero = result == 0;
    registers.flags.negative = (result & 0x80) != 0;
    return result;
}

// Return from Interrupt
fn rti(nes: &mut NesState) {
    let status_byte = pop(nes);
    set_status_from_byte(&mut nes.registers, status_byte);
    let pc_low = pop(nes) as u16;
    let pc_high = pop(nes) as u16;
    let pc = (pc_high << 8) + pc_low;
    nes.registers.pc = pc;
}

// Return from Subroutine
fn rts(nes: &mut NesState) {
    let pc_low = pop(nes) as u16;
    let pc_high = pop(nes) as u16;
    let pc = (pc_high << 8) + pc_low;
    nes.registers.pc = pc.wrapping_add(1);
}

// Subtract with Carry
fn sbc(registers: &mut Registers, data: u8) {
    // Preload the carry into bit 8
    let inverted_data = data ^ 0xFF;
    adc(registers, inverted_data);
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

// Addressing Modes
fn immediate(registers: &mut Registers) -> u16 {
    let address = registers.pc;
    registers.pc = registers.pc.wrapping_add(1);
    return address as u16;
}

fn zero_page(nes: &mut NesState) -> u16 {
    let pc = nes.registers.pc;
    let offset = read_byte(nes, pc);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
    return offset as u16;
}

fn zero_x(nes: &mut NesState) -> u16 {
    let pc = nes.registers.pc;
    let offset = read_byte(nes, pc).wrapping_add(nes.registers.x);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
    return offset as u16;
}

fn zero_y(nes: &mut NesState) -> u16 {
    let pc = nes.registers.pc;
    let offset = read_byte(nes, pc).wrapping_add(nes.registers.y);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
    return offset as u16;
}

fn absolute(nes: &mut NesState) -> u16 {
    let mut pc = nes.registers.pc;
    let address_low = read_byte(nes, pc);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
    pc = nes.registers.pc;
    let address_high = read_byte(nes, pc);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
    let address = ((address_high as u16) << 8) + (address_low as u16);
    return address as u16;
}

fn absolute_x(nes: &mut NesState) -> u16 {
    let mut pc = nes.registers.pc;
    let address_low = read_byte(nes, pc);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
    pc = nes.registers.pc;
    let address_high = read_byte(nes, pc);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
    let mut address = ((address_high as u16) << 8) + (address_low as u16);
    address = address.wrapping_add(nes.registers.x as u16);
    return address as u16;
}

fn absolute_y(nes: &mut NesState) -> u16 {
    let mut pc = nes.registers.pc;
    let address_low = read_byte(nes, pc);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
    pc = nes.registers.pc;
    let address_high = read_byte(nes, pc);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
    let mut address = ((address_high as u16) << 8) + (address_low as u16);
    address = address.wrapping_add(nes.registers.y as u16);
    return address as u16;
}

// Only used by jmp
fn indirect(nes: &mut NesState) -> u16 {
    let mut pc = nes.registers.pc;
    let mut indirect_low: u8 = read_byte(nes, pc);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
    pc = nes.registers.pc;
    let indirect_high: u8 = read_byte(nes, pc);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
    let mut indirect_address = ((indirect_high as u16) << 8) | (indirect_low as u16);

    let address_low = read_byte(nes, indirect_address);
    // This emulates a hardware bug. A jump to ($xxFF) reads the high byte from ($xx00),
    // not ($(xx+1)00) as one might expect.
    indirect_low = indirect_low.wrapping_add(1);
    indirect_address = ((indirect_high as u16) << 8) | (indirect_low as u16);
    let address_high = read_byte(nes, indirect_address);
    let address = ((address_high as u16) << 8) | (address_low as u16);

    return address as u16;
}

fn indexed_indirect_x(nes: &mut NesState) -> u16 {
    let pc = nes.registers.pc;
    let mut table_address = read_byte(nes, pc).wrapping_add(nes.registers.x);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
    let address_low = read_byte(nes, table_address as u16);
    table_address = table_address.wrapping_add(1);
    let address_high = read_byte(nes, table_address as u16);
    let address = ((address_high as u16) << 8) + (address_low as u16);
    return address as u16;
}

fn indirect_indexed_y(nes: &mut NesState) -> u16 {
    let pc = nes.registers.pc;
    let mut offset = read_byte(nes, pc);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
    let address_low = read_byte(nes, offset as u16);
    offset = offset.wrapping_add(1);
    let address_high = read_byte(nes, offset as u16);
    let mut address = ((address_high as u16) << 8) + (address_low as u16);
    address = address.wrapping_add(nes.registers.y as u16);
    return address as u16;
}

pub fn nmi_signal(nes: &NesState) -> bool {
    return ((nes.ppu.control & 0x80) & (nes.ppu.status & 0x80)) != 0;
}

pub fn service_nmi(nes: &mut NesState) {
    // Push PC and processor status to stack
    let pc_high = ((nes.registers.pc & 0xFF00) >> 8) as u8;
    let pc_low =  (nes.registers.pc & 0x00FF) as u8;
    push(nes, pc_high);
    push(nes, pc_low);
    let status_byte = status_as_byte(&mut nes.registers, false);
    push(nes, status_byte);
    // Set PC to NMI interrupt vector at FFFA/FFFB
    nes.registers.pc = read_byte(nes, 0xFFFA) as u16 + ((read_byte(nes, 0xFFFB) as u16) << 8);
}

pub fn process_instruction(nes: &mut NesState) {
    // Are conditions ripe for an NMI? Then do that instead.
    let current_nmi = nmi_signal(&nes);
    let last_nmi = nes.registers.flags.last_nmi;
    nes.registers.flags.last_nmi = current_nmi;
    if current_nmi && !last_nmi {
        service_nmi(nes);
        nes.registers.flags.last_nmi = current_nmi;
        return;
    }

    let pc = nes.registers.pc;
    let opcode = read_byte(nes, pc);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);

    match opcode {
        // Add with Carry
        0x69 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  adc(&mut nes.registers, byte)
        },
        0x65 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  adc(&mut nes.registers, byte)
        },
        0x75 => { let address = zero_x(nes);
                  let byte = read_byte(nes, address);
                  adc(&mut nes.registers, byte)
        },
        0x6D => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  adc(&mut nes.registers, byte)
        },
        0x7D => { let address = absolute_x(nes);
                  let byte = read_byte(nes, address);
                  adc(&mut nes.registers, byte)
        },
        0x79 => { let address = absolute_y(nes);
                  let byte = read_byte(nes, address);
                  adc(&mut nes.registers, byte)
        },
        0x61 => { let address = indexed_indirect_x(nes);
                  let byte = read_byte(nes, address);
                  adc(&mut nes.registers, byte)
        },
        0x71 => { let address = indirect_indexed_y(nes);
                  let byte = read_byte(nes, address);
                  adc(&mut nes.registers, byte)
        },

        // Logical AND
        0x29 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  and(&mut nes.registers, byte)
        },
        0x25 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  and(&mut nes.registers, byte)
        },
        0x35 => { let address = zero_x(nes);
                  let byte = read_byte(nes, address);
                  and(&mut nes.registers, byte)
        },
        0x2D => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  and(&mut nes.registers, byte)
        },
        0x3D => { let address = absolute_x(nes);
                  let byte = read_byte(nes, address);
                  and(&mut nes.registers, byte)
        },
        0x39 => { let address = absolute_y(nes);
                  let byte = read_byte(nes, address);
                  and(&mut nes.registers, byte)
        },
        0x21 => { let address = indexed_indirect_x(nes);
                  let byte = read_byte(nes, address);
                  and(&mut nes.registers, byte)
        },
        0x31 => { let address = indirect_indexed_y(nes);
                  let byte = read_byte(nes, address);
                  and(&mut nes.registers, byte)
        },

        // Arithmetic Shift Left
        0x0A => { let value = nes.registers.a;
                  nes.registers.a = asl(&mut nes.registers, value)
        },
        0x06 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  let result = asl(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0x16 => { let address = zero_x(nes);
                  let byte = read_byte(nes, address);
                  let result = asl(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0x0E => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  let result = asl(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0x1E => { let address = absolute_x(nes);
                  let byte = read_byte(nes, address);
                  let result = asl(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },

        // Branching
        0x90 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  bcc(&mut nes.registers, byte as i8)
        },
        0xB0 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  bcs(&mut nes.registers, byte as i8)
        },
        0xF0 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  beq(&mut nes.registers, byte as i8)
        },
        0x30 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  bmi(&mut nes.registers, byte as i8)
        },
        0xD0 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  bne(&mut nes.registers, byte as i8)
        },
        0x10 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  bpl(&mut nes.registers, byte as i8)
        },
        0x50 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  bvc(&mut nes.registers, byte as i8)
        },
        0x70 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  bvs(&mut nes.registers, byte as i8)
        },

        // Bit Test
        0x24 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  bit(&mut nes.registers, byte)
        },
        0x2C => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  bit(&mut nes.registers, byte)
        },

        // Break - Force Interrupt
        0x00 => brk(nes),

        // Clear Flags
        0x18 => clc(&mut nes.registers),
        0xD8 => cld(&mut nes.registers),
        0x58 => cli(&mut nes.registers),
        0xB8 => clv(&mut nes.registers),

        // Compare
        0xC9 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  cmp(&mut nes.registers, byte)
        },
        0xC5 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  cmp(&mut nes.registers, byte)
        },
        0xD5 => { let address = zero_x(nes);
                  let byte = read_byte(nes, address);
                  cmp(&mut nes.registers, byte)
        },
        0xCD => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  cmp(&mut nes.registers, byte)
        },
        0xDD => { let address = absolute_x(nes);
                  let byte = read_byte(nes, address);
                  cmp(&mut nes.registers, byte)
        },
        0xD9 => { let address = absolute_y(nes);
                  let byte = read_byte(nes, address);
                  cmp(&mut nes.registers, byte)
        },
        0xC1 => { let address = indexed_indirect_x(nes);
                  let byte = read_byte(nes, address);
                  cmp(&mut nes.registers, byte)
        },
        0xD1 => { let address = indirect_indexed_y(nes);
                  let byte = read_byte(nes, address);
                  cmp(&mut nes.registers, byte)
        },

        // Compare X
        0xE0 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  cpx(&mut nes.registers, byte)
        },
        0xE4 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  cpx(&mut nes.registers, byte)
        },
        0xEC => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  cpx(&mut nes.registers, byte)
        },

        // Compare Y
        0xC0 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  cpy(&mut nes.registers, byte)
        },
        0xC4 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  cpy(&mut nes.registers, byte)
        },
        0xCC => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  cpy(&mut nes.registers, byte)
        },

        // Decrement
        0xC6 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  let result = dec(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0xD6 => { let address = zero_x(nes);
                  let byte = read_byte(nes, address);
                  let result = dec(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0xCE => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  let result = dec(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0xDE => { let address = absolute_x(nes);
                  let byte = read_byte(nes, address);
                  let result = dec(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0xCA => dex(&mut nes.registers),
        0x88 => dey(&mut nes.registers),

        // Logical Exclusive OR
        0x49 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  eor(&mut nes.registers, byte)
        },
        0x45 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  eor(&mut nes.registers, byte)
        },
        0x55 => { let address = zero_x(nes);
                  let byte = read_byte(nes, address);
                  eor(&mut nes.registers, byte)
        },
        0x4D => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  eor(&mut nes.registers, byte)
        },
        0x5D => { let address = absolute_x(nes);
                  let byte = read_byte(nes, address);
                  eor(&mut nes.registers, byte)
        },
        0x59 => { let address = absolute_y(nes);
                  let byte = read_byte(nes, address);
                  eor(&mut nes.registers, byte)
        },
        0x41 => { let address = indexed_indirect_x(nes);
                  let byte = read_byte(nes, address);
                  eor(&mut nes.registers, byte)
        },
        0x51 => { let address = indirect_indexed_y(nes);
                  let byte = read_byte(nes, address);
                  eor(&mut nes.registers, byte)
        },

        // Increment
        0xE6 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  let result = inc(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0xF6 => { let address = zero_x(nes);
                  let byte = read_byte(nes, address);
                  let result = inc(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0xEE => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  let result = inc(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0xFE => { let address = absolute_x(nes);
                  let byte = read_byte(nes, address);
                  let result = inc(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0xE8 => inx(&mut nes.registers),
        0xC8 => iny(&mut nes.registers),

        // Jump
        0x4C => { let address = absolute(nes);
                  jmp(&mut nes.registers, address as u16)
        },
        0x6C => { let address = indirect(nes);
                  jmp(&mut nes.registers, address as u16)
        },

        // Jump to Subroutine
        0x20 => { let address = absolute(nes);
                  jsr(nes, address as u16)
        },

        // Load Accumulator
        0xA9 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  lda(&mut nes.registers, byte)
        },
        0xA5 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  lda(&mut nes.registers, byte)
        },
        0xB5 => { let address = zero_x(nes);
                  let byte = read_byte(nes, address);
                  lda(&mut nes.registers, byte)
        },
        0xAD => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  lda(&mut nes.registers, byte)
        },
        0xBD => { let address = absolute_x(nes);
                  let byte = read_byte(nes, address);
                  lda(&mut nes.registers, byte)
        },
        0xB9 => { let address = absolute_y(nes);
                  let byte = read_byte(nes, address);
                  lda(&mut nes.registers, byte)
        },
        0xA1 => { let address = indexed_indirect_x(nes);
                  let byte = read_byte(nes, address);
                  lda(&mut nes.registers, byte)
        },
        0xB1 => { let address = indirect_indexed_y(nes);
                  let byte = read_byte(nes, address);
                  lda(&mut nes.registers, byte)
        },

        // Load X
        0xA2 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  ldx(&mut nes.registers, byte)
        },
        0xA6 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  ldx(&mut nes.registers, byte)
        },
        0xB6 => { let address = zero_y(nes);
                  let byte = read_byte(nes, address);
                  ldx(&mut nes.registers, byte)
        },
        0xAE => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  ldx(&mut nes.registers, byte)
        },
        0xBE => { let address = absolute_y(nes);
                  let byte = read_byte(nes, address);
                  ldx(&mut nes.registers, byte)
        },

        // Load Y
        0xA0 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  ldy(&mut nes.registers, byte)
        },
        0xA4 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  ldy(&mut nes.registers, byte)
        },
        0xB4 => { let address = zero_x(nes);
                  let byte = read_byte(nes, address);
                  ldy(&mut nes.registers, byte)
        },
        0xAC => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  ldy(&mut nes.registers, byte)
        },
        0xBC => { let address = absolute_x(nes);
                  let byte = read_byte(nes, address);
                  ldy(&mut nes.registers, byte)
        },

        // Logical Shift Right
        0x4A => { let value = nes.registers.a;
                  nes.registers.a = lsr(&mut nes.registers, value)
        },
        0x46 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  let result = lsr(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0x56 => { let address = zero_x(nes);
                  let byte = read_byte(nes, address);
                  let result = lsr(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0x4E => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  let result = lsr(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0x5E => { let address = absolute_x(nes);
                  let byte = read_byte(nes, address);
                  let result = lsr(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },

        // NOP!
        0xEA => nop(),

        // Logical Inclusive OR
        0x09 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  ora(&mut nes.registers, byte)
        },
        0x05 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  ora(&mut nes.registers, byte)
        },
        0x15 => { let address = zero_x(nes);
                  let byte = read_byte(nes, address);
                  ora(&mut nes.registers, byte)
        },
        0x0D => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  ora(&mut nes.registers, byte)
        },
        0x1D => { let address = absolute_x(nes);
                  let byte = read_byte(nes, address);
                  ora(&mut nes.registers, byte)
        },
        0x19 => { let address = absolute_y(nes);
                  let byte = read_byte(nes, address);
                  ora(&mut nes.registers, byte)
        },
        0x01 => { let address = indexed_indirect_x(nes);
                  let byte = read_byte(nes, address);
                  ora(&mut nes.registers, byte)
        },
        0x11 => { let address = indirect_indexed_y(nes);
                  let byte = read_byte(nes, address);
                  ora(&mut nes.registers, byte)
        },

        // Push and Pop
        0x48 => pha(nes),
        0x08 => php(nes),
        0x68 => pla(nes),
        0x28 => plp(nes),

        // Rotate Left
        0x2A => { let value = nes.registers.a;
                  nes.registers.a = rol(&mut nes.registers, value)
        },
        0x26 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  let result = rol(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0x36 => { let address = zero_x(nes);
                  let byte = read_byte(nes, address);
                  let result = rol(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0x2E => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  let result = rol(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0x3E => { let address = absolute_x(nes);
                  let byte = read_byte(nes, address);
                  let result = rol(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },

        // Rotate Right
        0x6A => { let value = nes.registers.a;
                  nes.registers.a = ror(&mut nes.registers, value)
        },
        0x66 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  let result = ror(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0x76 => { let address = zero_x(nes);
                  let byte = read_byte(nes, address);
                  let result = ror(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0x6E => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  let result = ror(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },
        0x7E => { let address = absolute_x(nes);
                  let byte = read_byte(nes, address);
                  let result = ror(&mut nes.registers, byte);
                  write_byte(nes, address, result);
        },

        // Returns
        0x40 => rti(nes),
        0x60 => rts(nes),

        // Subtract with Carry
        0xE9 => { let address = immediate(&mut nes.registers);
                  let byte = read_byte(nes, address);
                  sbc(&mut nes.registers, byte)
        },
        0xE5 => { let address = zero_page(nes);
                  let byte = read_byte(nes, address);
                  sbc(&mut nes.registers, byte)
        },
        0xF5 => { let address = zero_x(nes);
                  let byte = read_byte(nes, address);
                  sbc(&mut nes.registers, byte)
        },
        0xED => { let address = absolute(nes);
                  let byte = read_byte(nes, address);
                  sbc(&mut nes.registers, byte)
        },
        0xFD => { let address = absolute_x(nes);
                  let byte = read_byte(nes, address);
                  sbc(&mut nes.registers, byte)
        },
        0xF9 => { let address = absolute_y(nes);
                  let byte = read_byte(nes, address);
                  sbc(&mut nes.registers, byte)
        },
        0xE1 => { let address = indexed_indirect_x(nes);
                  let byte = read_byte(nes, address);
                  sbc(&mut nes.registers, byte)
        },
        0xF1 => { let address = indirect_indexed_y(nes);
                  let byte = read_byte(nes, address);
                  sbc(&mut nes.registers, byte)
        },

        // Set Flags
        0x38 => sec(&mut nes.registers),
        0xF8 => sed(&mut nes.registers),
        0x78 => sei(&mut nes.registers),

        // Store Accumulator
        0x85 => { let address = zero_page(nes);
                  let result = sta(&mut nes.registers);
                  write_byte(nes, address, result);
        },
        0x95 => { let address = zero_x(nes);
                  let result = sta(&mut nes.registers);
                  write_byte(nes, address, result);
        },
        0x8D => { let address = absolute(nes);
                  let result = sta(&mut nes.registers);
                  write_byte(nes, address, result);
        },
        0x9D => { let address = absolute_x(nes);
                  let result = sta(&mut nes.registers);
                  write_byte(nes, address, result);
        },
        0x99 => { let address = absolute_y(nes);
                  let result = sta(&mut nes.registers);
                  write_byte(nes, address, result);
        },
        0x81 => { let address = indexed_indirect_x(nes);
                  let result = sta(&mut nes.registers);
                  write_byte(nes, address, result);
        },
        0x91 => { let address = indirect_indexed_y(nes);
                  let result = sta(&mut nes.registers);
                  write_byte(nes, address, result);
        },

        // Store X
        0x86 => { let address = zero_page(nes);
                  let result = stx(&mut nes.registers);
                  write_byte(nes, address, result);
        },
        0x96 => { let address = zero_y(nes);
                  let result = stx(&mut nes.registers);
                  write_byte(nes, address, result);
        },
        0x8E => { let address = absolute(nes);
                  let result = stx(&mut nes.registers);
                  write_byte(nes, address, result);
        },

        // Store Y
        0x84 => { let address = zero_page(nes);
                  let result = sty(&mut nes.registers);
                  write_byte(nes, address, result);
        },
        0x94 => { let address = zero_x(nes);
                  let result = sty(&mut nes.registers);
                  write_byte(nes, address, result);
        },
        0x8C => { let address = absolute(nes);
                  let result = sty(&mut nes.registers);
                  write_byte(nes, address, result);
        },

        0xAA => tax(&mut nes.registers),
        0xA8 => tay(&mut nes.registers),
        0xBA => tsx(&mut nes.registers),
        0x8A => txa(&mut nes.registers),
        0x9A => txs(&mut nes.registers),
        0x98 => tya(&mut nes.registers),

        // Undefined Weirdness
        _ => println!("Undefined opcode: {:X}", opcode)
    }
}
