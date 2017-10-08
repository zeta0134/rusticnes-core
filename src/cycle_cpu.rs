// Documentation for this 6502 implementation came from many sources, but the following
// two guides served as the primary inspiration:
// http://www.llx.com/~nparker/a2/opcodes.html - For opcode decoding structure
// http://nesdev.com/6502_cpu.txt - for information on cycle timings for each addressing mode

use addressing;
use cpu;
use memory::read_byte;
use nes::NesState;
use opcodes;

pub struct CpuState {
  pub tick: u8,
  pub opcode: u8,
  pub data1: u8,
  pub data2: u8,
  pub temp_address: u16,
  pub service_routine_active: bool,
  pub nmi_requested: bool,
  pub last_nmi: bool,
}

impl CpuState {
  pub fn new() -> CpuState{
    return CpuState {
      tick: 0,
      opcode: 0,
      data1: 0,
      data2: 0,
      temp_address: 0,
      service_routine_active: false,
      nmi_requested: false,
      last_nmi: false,
    }
  }
}

pub fn nmi_signal(nes: &NesState) -> bool {
    return ((nes.ppu.control & 0x80) & (nes.ppu.status & 0x80)) != 0;
}

pub fn irq_signal(nes: &NesState) -> bool {
  if nes.registers.flags.interrupts_disabled {
    return false;
  } else {
    return nes.apu.irq_signal();
  }
}

pub fn poll_for_interrupts(nes: &mut NesState) {
  let current_nmi = nmi_signal(&nes);
  let last_nmi = nes.registers.flags.last_nmi;
  nes.registers.flags.last_nmi = current_nmi;
  if current_nmi && !last_nmi {
    nes.cpu.nmi_requested = true;
  }
}

pub fn interrupt_requested(nes: &NesState) -> bool {
  return nes.cpu.nmi_requested || irq_signal(&nes);
}

pub fn alu_block(nes: &mut NesState, addressing_mode_index: u8, opcode_index: u8) {
  let addressing_mode = match addressing_mode_index {
    // Zero Page Mode
    0b000 => &addressing::INDEXED_INDIRECT_X,
    0b001 => &addressing::ZERO_PAGE,
    0b010 => &addressing::IMMEDIATE,
    0b011 => &addressing::ABSOLUTE,
    0b100 => &addressing::INDIRECT_INDEXED_Y,
    0b101 => &addressing::ZERO_PAGE_INDEXED_X,
    0b110 => &addressing::ABSOLUTE_INDEXED_Y,
    0b111 => &addressing::ABSOLUTE_INDEXED_X,

    // Not implemented yet
    _ => &addressing::UNIMPLEMENTED,
  };

  match opcode_index {
    0b000 => {(addressing_mode.read)(nes, opcodes::ora)},
    0b001 => {(addressing_mode.read)(nes, opcodes::and)},
    0b010 => {(addressing_mode.read)(nes, opcodes::eor)},
    0b011 => {(addressing_mode.read)(nes, opcodes::adc)},
    0b100 => {(addressing_mode.write)(nes, opcodes::sta)},
    0b101 => {(addressing_mode.read)(nes, opcodes::lda)},
    0b110 => {(addressing_mode.read)(nes, opcodes::cmp)},
    0b111 => {(addressing_mode.read)(nes, opcodes::sbc)},
    _ => ()
  };
}

pub fn rmw_block(nes: &mut NesState, addressing_mode_index: u8, opcode_index: u8) {
  // First, handle some block 10 opcodes that break the mold
  match nes.cpu.opcode {
    // Assorted NOPs
    0x82 | 0xC2 | 0xE2 => (addressing::IMMEDIATE.read) (nes, opcodes::nop_read),
    0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xEA | 0xFA => addressing::implied(nes, opcodes::nop),
    // Certain opcodes may be vital to your success. THESE opcodes are not.
    0x02 | 0x22 | 0x42 | 0x62 | 0x12 | 0x32 | 0x52 | 0x72 | 0x92 | 0xB2 | 0xD2 | 0xF2 => {
      // HALT the CPU. It died, jim.
      if nes.cpu.tick < 10 {
        println!("STP opcode encountered: {}", nes.cpu.opcode);
        println!("Proceeding to lock up CPU. Goodbye, cruel world!");
      }
      nes.cpu.tick = 10;
    },
    0xA2 => {(addressing::IMMEDIATE.read)(nes, opcodes::ldx)},
    0x8A => addressing::implied(nes, opcodes::txa),
    0xAA => addressing::implied(nes, opcodes::tax),
    0xCA => addressing::implied(nes, opcodes::dex),
    0x9A => addressing::implied(nes, opcodes::txs),
    0xBA => addressing::implied(nes, opcodes::tsx),
    0x96 => {(addressing::ZERO_PAGE_INDEXED_Y.write)(nes, opcodes::stx)},
    0xB6 => {(addressing::ZERO_PAGE_INDEXED_Y.read)(nes, opcodes::ldx)},
    0xBE => {(addressing::ABSOLUTE_INDEXED_Y.read)(nes, opcodes::ldx)},
    _ => {
      let addressing_mode = match addressing_mode_index {
        // Zero Page Mode
        0b001 => &addressing::ZERO_PAGE,
        0b010 => &addressing::ACCUMULATOR, // Note: masked for 8A, AA, CA, and EA above
        0b011 => &addressing::ABSOLUTE,
        0b101 => &addressing::ZERO_PAGE_INDEXED_X,
        0b111 => &addressing::ABSOLUTE_INDEXED_X,

        // Not implemented yet
        _ => &addressing::UNIMPLEMENTED,
      };

      match opcode_index {
        0b000 => {(addressing_mode.modify)(nes, opcodes::asl)},
        0b001 => {(addressing_mode.modify)(nes, opcodes::rol)},
        0b010 => {(addressing_mode.modify)(nes, opcodes::lsr)},
        0b011 => {(addressing_mode.modify)(nes, opcodes::ror)},
        0b100 => {(addressing_mode.write)(nes, opcodes::stx)},
        0b101 => {(addressing_mode.read)(nes, opcodes::ldx)},
        0b110 => {(addressing_mode.modify)(nes, opcodes::dec)},
        0b111 => {(addressing_mode.modify)(nes, opcodes::inc)},
        _ => ()
      };
    }
  };
}

pub fn control_block(nes: &mut NesState) {
  // Branch instructions are of the form xxy10000
  if (nes.cpu.opcode & 0b1_1111) == 0b1_0000 {
    return opcodes::branch(nes);
  }

  // Everything else is pretty irregular, so we'll just match the whole opcode
  match nes.cpu.opcode {
    0x00 => opcodes::brk(nes),
    0x80 => (addressing::IMMEDIATE.read)  (nes, opcodes::nop_read),

    // Opcodes with similar addressing modes
    0xA0 => (addressing::IMMEDIATE.read)  (nes, opcodes::ldy),
    0xC0 => (addressing::IMMEDIATE.read)  (nes, opcodes::cpy),
    0xE0 => (addressing::IMMEDIATE.read)  (nes, opcodes::cpx),
    0x24 => (addressing::ZERO_PAGE.read)  (nes, opcodes::bit),
    0x84 => (addressing::ZERO_PAGE.write) (nes, opcodes::sty),
    0xA4 => (addressing::ZERO_PAGE.read)  (nes, opcodes::ldy),
    0xC4 => (addressing::ZERO_PAGE.read)  (nes, opcodes::cpy),
    0xE4 => (addressing::ZERO_PAGE.read)  (nes, opcodes::cpx),
    0x2C => (addressing::ABSOLUTE.read)  (nes, opcodes::bit),
    0x8C => (addressing::ABSOLUTE.write) (nes, opcodes::sty),
    0xAC => (addressing::ABSOLUTE.read)  (nes, opcodes::ldy),
    0xCC => (addressing::ABSOLUTE.read)  (nes, opcodes::cpy),
    0xEC => (addressing::ABSOLUTE.read)  (nes, opcodes::cpx),
    0x94 => (addressing::ZERO_PAGE_INDEXED_X.write) (nes, opcodes::sty),
    0xB4 => (addressing::ZERO_PAGE_INDEXED_X.read)  (nes, opcodes::ldy),
    0xBC => (addressing::ABSOLUTE_INDEXED_X.read)  (nes, opcodes::ldy),

    0x4C => opcodes::jmp_absolute(nes),
    0x6C => opcodes::jmp_indirect(nes),

    0x08 => opcodes::php(nes),
    0x28 => opcodes::plp(nes),
    0x48 => opcodes::pha(nes),
    0x68 => opcodes::pla(nes),

    0x20 => opcodes::jsr(nes),

    _ => {
      // Unimplemented, fall back on old behavior
      nes.registers.pc = nes.registers.pc.wrapping_sub(1);
      cpu::process_instruction(nes);
      nes.cpu.tick = 0;
    }
  };
}

pub fn run_one_clock(nes: &mut NesState) {
  nes.cpu.tick += 1;

  // The ordering of these checks may seem a bit strange. The 6502 polls for interrupts
  // at the START of each cycle, not at the end. This means that whether an interrupt is
  // serviced is determined right before the last cycle of a give instruction, not after
  // the last cycle as one might expect.

  if nes.cpu.tick == 1 && interrupt_requested(&nes) {
    nes.cpu.service_routine_active = true;
  }

  poll_for_interrupts(nes);

  if nes.cpu.service_routine_active {
    return opcodes::service_interrupt(nes);
  }

  // Universal behavior for every opcode
  if nes.cpu.tick == 1 {
    // Fetch opcode from memory
    let pc = nes.registers.pc;
    nes.cpu.opcode = read_byte(nes, pc);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
    return; // all done
  }

  // Decode this opcode into its component parts
  let logic_block = nes.cpu.opcode & 0b0000_0011;
  let addressing_mode_index = (nes.cpu.opcode & 0b0001_1100) >> 2;
  let opcode_index = (nes.cpu.opcode & 0b1110_0000) >> 5;

  match logic_block {
    0b00 => control_block(nes),
    0b01 => alu_block(nes, addressing_mode_index, opcode_index),
    0b10 => rmw_block(nes, addressing_mode_index, opcode_index),
    _ => {
      // We don't have this block implemented! Fall back to old behavior.
      nes.registers.pc = nes.registers.pc.wrapping_sub(1);
      cpu::process_instruction(nes);
      nes.cpu.tick = 0;
    }
  }
}