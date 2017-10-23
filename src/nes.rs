use apu::ApuState;
use cartridge;
use cycle_cpu;
use cycle_cpu::CpuState;
use cycle_cpu::Registers;
use memory;
use memory::CpuMemory;
use ppu::PpuState;
use mmc::mapper::Mapper;

use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::io::Write;

pub struct NesState {
    pub apu: ApuState,
    pub cpu: CpuState,
    pub memory: CpuMemory,
    pub ppu: PpuState,
    pub registers: Registers,
    pub master_clock: u64,
    pub p1_input: u8,
    pub p1_data: u8,
    pub p2_input: u8,
    pub p2_data: u8,
    pub input_latch: bool,
    pub mapper: Box<Mapper>,
}

impl NesState {
    pub fn new(m: Box<Mapper>) -> NesState {
        return NesState {
            apu: ApuState::new(),
            cpu: CpuState::new(),
            memory: CpuMemory::new(),
            ppu: PpuState::new(),
            registers: Registers::new(),
            master_clock: 0,
            p1_input: 0,
            p1_data: 0,
            p2_input: 0,
            p2_data: 0,
            input_latch: false,
            mapper: m,
        }
    }

    pub fn from_rom(cart_data: &[u8]) -> Result<NesState, String> {
        let nes_header = cartridge::extract_header(cart_data);
        cartridge::print_header_info(nes_header);
        let mapper = cartridge::load_from_cartridge(nes_header, cart_data);
        let mut nes = NesState::new(mapper);

        nes.power_on();

        return Ok(nes);
    }

    pub fn power_on(&mut self) {
        // Initialize CPU register state for power-up sequence
        self.registers.a = 0;
        self.registers.y = 0;
        self.registers.x = 0;
        self.registers.s = 0xFD;

        self.registers.set_status_from_byte(0x34);

        let pc_low = memory::read_byte(self, 0xFFFC);
        let pc_high = memory::read_byte(self, 0xFFFD);
        self.registers.pc = pc_low as u16 + ((pc_high as u16) << 8);
    }

    pub fn reset(&mut self) {
        self.registers.s = self.registers.s.wrapping_sub(3);
        self.registers.flags.interrupts_disabled = true;
        // Silence the APU
        memory::write_byte(self, 0x4015, 0);

        let pc_low = memory::read_byte(self, 0xFFFC);
        let pc_high = memory::read_byte(self, 0xFFFD);
        self.registers.pc = pc_low as u16 + ((pc_high as u16) << 8);
    }

    pub fn cycle(&mut self) {
        cycle_cpu::run_one_clock(self);
        self.master_clock = self.master_clock + 12;
        // Three PPU clocks per every 1 CPU clock
        self.ppu.clock(&mut *self.mapper);
        self.ppu.clock(&mut *self.mapper);
        self.ppu.clock(&mut *self.mapper);
        self.apu.clock_apu(&mut *self.mapper);
    }

    pub fn step(&mut self) {
        // Always run at least one cycle
        self.cycle();
        let mut i = 0;
        // Continue until either we loop back around to cycle 0 (a new instruction)
        // or this instruction has failed to reset (encountered a STP or an opcode bug)
        while self.cpu.tick >= 1 && i < 10 {
            self.cycle();
            i += 1;
        }
    }

    pub fn run_until_hblank(&mut self) {
        let old_scanline = self.ppu.current_scanline;
        while old_scanline == self.ppu.current_scanline {
            self.step();
        }
    }

    pub fn run_until_vblank(&mut self) {
        while self.ppu.current_scanline == 242 {
            self.step();
        }
        while self.ppu.current_scanline != 242 {
            self.step();
        }
    }

    pub fn write_sram(&mut self, file_path: &str) {
        if self.mapper.has_sram() {
            let file = File::create(file_path);
            match file {
                Err(why) => {
                    println!("Couldn't open {}: {}", file_path, why.description());
                },
                Ok(mut file) => {
                    let _ = file.write_all(&self.mapper.get_sram());
                },
            };
        }
    }

    pub fn read_sram(&mut self, file_path: &str) {
        let file = File::open(file_path);
        match file {
            Err(why) => {
                println!("Couldn't open {}: {}", file_path, why.description());
                return;
            },
            Ok(_) => (),
        };
        // Read the whole thing
        let mut sram_data = Vec::new();
        match file.unwrap().read_to_end(&mut sram_data) {
            Err(why) => {
                println!("Couldn't read data: {}", why.description());
                return;
            },
            Ok(bytes_read) => {
                if bytes_read != self.mapper.get_sram().len() {
                    println!("SRAM size mismatch, expected {} bytes but file is {} bytes!", bytes_read, self.mapper.get_sram().len());
                } else {
                    self.mapper.load_sram(sram_data);
                }
            }
        }
    }
}

pub fn open_file(file_path: &str) -> Result<NesState, String> {
    let file = File::open(file_path);
    match file {
        Err(why) => {
            return Err(format!("Couldn't open {}: {}", file_path, why.description()));
        },
        Ok(_) => (),
    };
    // Read the whole thing
    let mut cartridge = Vec::new();
    match file.unwrap().read_to_end(&mut cartridge) {
        Err(why) => {
            return Err(format!("Couldn't read file data"));
        },
        Ok(bytes_read) => {
            println!("Data read successfully: {}", bytes_read);
            return NesState::from_rom(&cartridge);
        },
    };
}