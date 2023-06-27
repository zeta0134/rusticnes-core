// A very simple Mapper with no esoteric features or bank switching.
// Reference capabilities: https://wiki.nesdev.com/w/index.php/NROM

use fds::FdsFile;

use mmc::mapper::*;
use mmc::mirroring;

pub struct FdsMapper {
    bios_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    chr: Vec<u8>,

    bios_loaded: bool,

    mirroring: Mirroring,
    vram: Vec<u8>,

    timer_reload_value: u16,
    timer_current_value: u16,
    timer_enabled: bool,
    timer_repeat: bool,
    timer_pending: bool,
    enable_disk_registers: bool,
    enable_sound_registers: bool,
}

impl FdsMapper {
    pub fn from_fds(_fds: FdsFile) -> Result<FdsMapper, String> {
        return Ok(FdsMapper {
            bios_rom: vec![0u8; 0x2000],
            prg_ram: vec![0u8; 0x8000],
            chr: vec![0u8; 0x2000],
            bios_loaded: false,
            mirroring: Mirroring::Horizontal,
            vram: vec![0u8; 0x1000],

            timer_reload_value: 0,
            timer_current_value: 0,
            timer_enabled: false,
            timer_repeat: false,
            timer_pending: false,
            enable_disk_registers: true,
            enable_sound_registers: true,
        });
    }
}

impl Mapper for FdsMapper {
    fn print_debug_status(&self) {
        println!("======= FDS =======");
        println!("Mirroring Mode: {}", mirroring_mode_name(self.mirroring));
        println!("====================");
    }

    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }

    fn clock_cpu(&mut self) {
        if self.timer_enabled {
            if self.timer_current_value == 0 {
                self.timer_pending = true;
                self.timer_current_value = self.timer_reload_value;
                if !self.timer_repeat {
                    self.timer_enabled = false;
                }
            } else {
                self.timer_current_value -= 1;
            }
        }
    }

    fn irq_flag(&self) -> bool {
        return self.timer_pending;
    }

    fn read_cpu(&mut self, address: u16) -> Option<u8> {
        match address {
            0x4030 => {
                // TODO - Incomplete!
                self.timer_pending = false;
                Some(0x00)
            },
            _ => {self.debug_read_cpu(address)}
        }
    }
    
    fn debug_read_cpu(&self, address: u16) -> Option<u8> {
        match address {
            0x6000 ..= 0xDFFF => {Some(self.prg_ram[address as usize - 0x6000])},
            0xE000 ..= 0xFFFF => {Some(self.bios_rom[address as usize - 0xE000])},
            _ => None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        if address & 0xFF00 == 0x4000 {
            println!("Wrote 0x{:02X} to ${:04X}", data, address);
        }
        match address {
            0x6000 ..= 0xDFFF => {self.prg_ram[address as usize - 0x6000] = data;},
            0x4020 => {self.timer_reload_value = (self.timer_reload_value & 0xFF00) | (data as u16)},
            0x4021 => {self.timer_reload_value = (self.timer_reload_value & 0x00FF) | ((data as u16) << 8)},
            0x4022 => {
                if self.enable_disk_registers {
                    self.timer_repeat =  (data & 0b0000_0001) != 0;
                    self.timer_enabled = (data & 0b0000_0010) != 0;
                    if !self.timer_enabled {
                        self.timer_pending = false;
                    }
                }
            },
            0x4023 => {
                self.enable_disk_registers = (data & 0b0000_0001) != 0;
                self.enable_sound_registers = (data & 0b0000_0010) != 0;
                if !self.enable_disk_registers {
                    self.timer_pending = false;
                    self.timer_enabled = false;
                }
            },
            _ => {}
        }
    }

    fn debug_read_ppu(&self, address: u16) -> Option<u8> {
        match address {
            0x0000 ..= 0x1FFF => return Some(self.chr[address as usize]),
            0x2000 ..= 0x3FFF => return match self.mirroring {
                Mirroring::Horizontal => Some(self.vram[mirroring::horizontal_mirroring(address) as usize]),
                Mirroring::Vertical   => Some(self.vram[mirroring::vertical_mirroring(address) as usize]),
                _ => None
            },
            _ => return None
        }
    }

    fn write_ppu(&mut self, address: u16, data: u8) {
        match address {
            0x0000 ..= 0x1FFF => {self.chr[address as usize] = data;},
            0x2000 ..= 0x3FFF => match self.mirroring {
                Mirroring::Horizontal => self.vram[mirroring::horizontal_mirroring(address) as usize] = data,
                Mirroring::Vertical   => self.vram[mirroring::vertical_mirroring(address) as usize] = data,
                _ => {}
            },
            _ => {}
        }
    }

    fn needs_bios(&self) -> bool {
        return !self.bios_loaded;
    }

    fn load_bios(&mut self, bios_rom: Vec<u8>) {
        if bios_rom.len() >= 8192 {
            self.bios_rom = bios_rom.clone();
            self.bios_loaded = true;
        } else {
            println!("FDS bios provided is less than 8k in length! Bad dump?")
        }
    }
}
