use cycle_cpu::Registers;
use opcodes;

// Note: Opcode names follow the undefined opcodes tabke here:
// https://wiki.nesdev.com/w/index.php/CPU_unofficial_opcodes

// Shift left and inclusive OR A
pub fn slo(registers: &mut Registers, data: u8) -> u8 {
  let result = opcodes::asl(registers, data);
  opcodes::ora(registers, result);
  return result;
}

// Rotate left, then AND A
pub fn rla(registers: &mut Registers, data: u8) -> u8 {
  let result = opcodes::rol(registers, data);
  opcodes::and(registers, result);
  return result;
}

// Shift right, then Exclisive OR A
pub fn sre(registers: &mut Registers, data: u8) -> u8 {
  let result = opcodes::lsr(registers, data);
  opcodes::eor(registers, result);
  return result;
}

// Rotate right, then ADC result with A
pub fn rra(registers: &mut Registers, data: u8) -> u8 {
  let result = opcodes::ror(registers, data);
  opcodes::adc(registers, result);
  return result;
}

pub fn sax(registers: &mut Registers) -> u8 {
  let result = registers.a & registers.x;
  return result;
}

pub fn lax(registers: &mut Registers, data: u8) {
  registers.a = data;
  registers.x = data;
}

// Decrement and compare
pub fn dcp(registers: &mut Registers, data: u8) -> u8 {
  let result = opcodes::dec(registers, data);
  opcodes::cmp(registers, result);
  return result;
}

// Increment and subtract w/ carry
pub fn isc(registers: &mut Registers, data: u8) -> u8 {
  let result = opcodes::inc(registers, data);
  opcodes::sbc(registers, result);
  return result;
}




