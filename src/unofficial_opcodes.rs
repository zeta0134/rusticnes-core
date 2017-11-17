use addressing;
use cycle_cpu::Registers;
use nes::NesState;
use memory::read_byte;
use memory::write_byte;
use opcodes;

// Note: Opcode names follow the undefined opcodes tabke here:
// https://wiki.nesdev.com/w/index.php/CPU_unofficial_opcodes

// Shift left and load
pub fn slo(registers: &mut Registers, data: u8) -> u8 {
  let result = opcodes::asl(registers, data);
  opcodes::ora(registers, data);
  return result;
}

