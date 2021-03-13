// This is an extremely simple collection of routines that can produce
// arbitrary 6502 code as a byte string. Especially useful for mappers
// which need to include arbitrary code that isn't provided by the file
// for whatever reason, but possibly also handy for units tests down
// the line.

pub enum AddressingMode {
    Accumulator,
    Immediate(u8),
    ZeroPage(u8),
    ZeroPageX(u8),
    ZeroPageY(u8),
    Relative(i8),
    RelativeLabel(String),
    Absolute(u16),
    AbsoluteLabel(String),
    AbsoluteX(u16),
    AbsoluteLabelX(String),
    AbsoluteY(u16),
    AbsoluteLabelY(String),
    Indirect(u16),
    IndexedIndirect(u8),
    IndirectIndexed(u8),
}

pub enum Opcode {
    Adc(AddressingMode),
    And(AddressingMode),
    Asl(AddressingMode),
    Bcc(AddressingMode),
    Bcs(AddressingMode),
    Beq(AddressingMode),
    Bit(AddressingMode),
    Bmi(AddressingMode),
    Bne(AddressingMode),
    Bpl(AddressingMode),
    Brk,
    Bvc(AddressingMode),
    Bvs(AddressingMode),
    Clc,
    Cli,
    Clv,
    Cmp(AddressingMode),
    Cpx(AddressingMode),
    Cpy(AddressingMode),
    Dec(AddressingMode),
    Dex(AddressingMode),
    Dey(AddressingMode),
    Eor(AddressingMode),
    Inc(AddressingMode),
    Inx(AddressingMode),
    Iny(AddressingMode),
    Jmp(AddressingMode),
    Jsr(AddressingMode),
    Lda(AddressingMode),
    Ldx(AddressingMode),
    Ldy(AddressingMode),
    Lsr(AddressingMode),
    Nop,
    Ora(AddressingMode),
    Pha,
    Php,
    Pla,
    Plp,
    Rol(AddressingMode),
    Ror(AddressingMode),
    Rti,
    Rts,
    Sbc(AddressingMode),
    Sec,
    Sei,
    Sta(AddressingMode),
    Stx(AddressingMode),
    Sty(AddressingMode),
    Tax,
    Tay,
    Tsx,
    Txa,
    Txs,
    Tya,   
}

// Utilities to help compact the opcode decoding block
fn low(word: u16) -> u8 {
    return (word & 0x00FF) as u8;
}

fn high(word: u16) -> u8 {
    return ((word & 0xFF00) >> 8) as u8;
}

pub fn opcode_bytes(opcode: Opcode) -> Result<Vec<u8>, String> {
    match opcode {
        Opcode::Brk => {Ok(vec![0x00])},
        Opcode::Lda(AddressingMode::Immediate(byte)) => {Ok(vec![0xA9, byte])},
        Opcode::Sta(AddressingMode::Absolute(address)) => {Ok(vec![0x8D, low(address), high(address)])},
        _ => {Err("Unimplemented!".to_string())}
    }
}

pub fn assemble(opcodes: Vec<Opcode>) -> Result<Vec<u8>, String> {
    let mut bytes: Vec<u8> = Vec::new();
    for opcode in opcodes {
        bytes.extend(opcode_bytes(opcode)?);
    }
    return Ok(bytes);
}