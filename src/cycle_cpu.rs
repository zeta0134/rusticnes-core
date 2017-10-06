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

pub fn service_interrupt(nes: &mut NesState) {
  match nes.cpu.tick {
    1 => {
      let pc = nes.registers.pc;
      let _ = read_byte(nes, pc);
      nes.cpu.opcode = 0x00; // Force next opcode to BRK
    },
    2 => {
      // Fetch data byte from memory (and discard it)
      let pc = nes.registers.pc;
      nes.cpu.data1 = read_byte(nes, pc);
    },
    3 => {
      let pc_high = ((nes.registers.pc & 0xFF00) >> 8) as u8;
      cpu::push(nes, pc_high);
    },
    4 => {
      let pc_low =  (nes.registers.pc & 0x00FF) as u8;
      cpu::push(nes, pc_low);
    },
    5 => {
      // At this point, NMI always takes priority, otherwise we run
      // an IRQ
      if nes.cpu.nmi_requested {
        nes.cpu.nmi_requested = false;
        nes.cpu.temp_address = 0xFFFA;
      } else {
        nes.cpu.temp_address = 0xFFFE;
      }
      let status_byte = cpu::status_as_byte(&mut nes.registers, false);
      cpu::push(nes, status_byte);
    },
    6 => {
      // Read PCL from interrupt vector
      let interrupt_vector = nes.cpu.temp_address;
      nes.registers.pc = (nes.registers.pc & 0xFF00) | read_byte(nes, interrupt_vector) as u16;
      // Disable IRQ handling (to be re-enabled by software, usually during RTI)
      nes.registers.flags.interrupts_disabled = true;
    },
    7 => {
      // Read PCH from interrupt vector
      let interrupt_vector = nes.cpu.temp_address;
      nes.registers.pc = (nes.registers.pc & 0x00FF) | ((read_byte(nes, interrupt_vector + 1) as u16) << 8);
      // All done!
      nes.cpu.tick = 0;
      nes.cpu.service_routine_active = false;
    },
    _ => ()
  }
}

pub fn brk(nes: &mut NesState) {
  // BRK's first cycle is the same as any ordinary instruction.
  match nes.cpu.tick {
    2 => {
      let pc = nes.registers.pc;
      let _ = read_byte(nes, pc);
      nes.registers.pc = nes.registers.pc.wrapping_add(1);
    },
    3 ... 4 => service_interrupt(nes),
    5 => {
      // At this point, NMI always takes priority, otherwise we run
      // an IRQ. This is the source of the BRK hijack quirk / bug.
      if nes.cpu.nmi_requested {
        nes.cpu.nmi_requested = false;
        nes.cpu.temp_address = 0xFFFA;
      } else {
        nes.cpu.temp_address = 0xFFFE;
      }
      // Here we set the B flag to signal a BRK, even if we end up servicing an NMI instead.
      let status_byte = cpu::status_as_byte(&mut nes.registers, true);
      cpu::push(nes, status_byte);
    },
    6 ... 7 => service_interrupt(nes),
    _ => ()
  }
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
    return service_interrupt(nes);
  }

  // Universal behavior for every opcode
  if nes.cpu.tick == 1 {
    // Fetch opcode from memory
    let pc = nes.registers.pc;
    nes.cpu.opcode = read_byte(nes, pc);
    nes.registers.pc = nes.registers.pc.wrapping_add(1);
    return; // all done
  }

  // Several control instructions are unique and irregular, so detect and process those here
  match nes.cpu.opcode {
    0x00 => {return brk(nes);},
    _ => (),
  }

  // The remaining opcodes follow a somewhat regular pattern.
  // Every instruction performs this read, regardless of whether
  // the data is used.
  if nes.cpu.tick == 2 {
    // Fetch data byte from memory
    let pc = nes.registers.pc;
    nes.cpu.data1 = read_byte(nes, pc);
  }

  // Branch instructions are of the form xxy10000
  if (nes.cpu.opcode & 0b1_1111) == 0b1_0000 {
    return opcodes::branch(nes);
  }

  // Decode this opcode into its component parts
  let logic_block = nes.cpu.opcode & 0b0000_0011;
  let addressing_mode_index = (nes.cpu.opcode & 0b0001_1100) >> 2;
  let opcode_index = (nes.cpu.opcode & 0b1110_0000) >> 5;

  match logic_block {
    0b01 => {
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
    },
    _ => {
      // We don't have this block implemented! Fall back to old behavior.
      nes.registers.pc = nes.registers.pc.wrapping_sub(1);
      cpu::process_instruction(nes);
      nes.cpu.tick = 0;
    }
  }
}