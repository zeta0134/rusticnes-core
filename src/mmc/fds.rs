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

    write_buffer: u8,
    read_buffer: u8,
    expansion_port_buffer: u8,

    disk_image: Vec<u8>,
    head_position: usize,
    rewinding: bool,
    motor_on: bool,
    disk_irq_enabled: bool,
    disk_irq_pending: bool,
    byte_transfer_flag: bool,
    write_mode: bool,
    motor_delay_counter: i16,
    disk_ready_flag: bool,
    transfer_reset_flag: bool,
    checksum: u16,

    old_4025: u8,
}

impl FdsMapper {
    pub fn from_fds(fds: FdsFile) -> Result<FdsMapper, String> {
        // FOR NOW, use just the first disk and ignore the rest
        let expanded_disk = expand_disk_image(&fds.disk_sides[0]);

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

            write_buffer: 0,
            read_buffer: 0,
            expansion_port_buffer: 0,

            disk_image: expanded_disk,
            head_position: 0,
            rewinding: false,
            motor_on: false,
            disk_irq_enabled: false,
            disk_irq_pending: false,
            byte_transfer_flag: false,
            write_mode: false,
            motor_delay_counter: 448,
            disk_ready_flag: false,
            transfer_reset_flag: false,
            checksum: 0,

            old_4025: 0,
        });
    }

    fn clock_timer_irq(&mut self) {
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

    fn update_disk_motor(&mut self) {
        // The disk ready flag is set if the program has requested a transfer AND
        // the drive head has made it to the beginning of the disk. Two conditions
        // will un-set this flag:
        // The motor reaches the end of the disk
        // The program disables the drive motor (the disk has to complete that revolution
        // and we don't want the transfer to "start" in the middle of a file or something)
        if self.motor_on && self.head_position == 0 {
            self.disk_ready_flag = true;
        }

        if self.motor_on || self.head_position != 0 {
            if self.motor_delay_counter <= 0 {
                self.motor_delay_counter += 448;
                if self.rewinding {
                    self.rewind_disk();
                } else {
                    self.advance_disk();
                }
            } else {
                self.motor_delay_counter -= 3;
            }
        }
    }

    fn rewind_disk(&mut self) {
        // I couldn't find documentation on this, but I had some anectodal reports
        // that the drive seems to rewind in about one second. Since it takes around
        // 7 seconds to move along the entire disk, I'm choosing to rewing at 7x
        // speed for now. 
        // TODO: obtain hardware and measure this more precisely
        if self.head_position <= 10 {
            self.rewinding = false;
            self.head_position = 0; // Click!
        } else {
            self.head_position -= 7;
        }
    }

    fn advance_disk(&mut self) {
        self.head_position += 1;
        if self.head_position == 81919 {
            self.rewinding = true;
            self.disk_ready_flag = false;
        } else {
            // Don't actually read bytes or generate IRQs if not in an active state
            if self.disk_ready_flag {
                if self.write_mode {
                    self.handle_write_mode_byte();
                } else {
                    self.handle_read_mode_byte();   
                }
            }
        }
    }

    fn handle_read_mode_byte(&mut self) {
        let current_data_byte = self.disk_image[self.head_position];

        // Try to cheat the checksum!
        // TODO: compute a real checksum here
        if current_data_byte != 0 {
            if current_data_byte == 98 {
                self.checksum = 0;
            } else {
                self.checksum = 77;
            }    
        }

        if self.transfer_reset_flag {
            if current_data_byte == 0x80 {
                self.transfer_reset_flag = false;
            }
        } else {
            self.read_buffer = current_data_byte;
            self.byte_transfer_flag = true;
            if self.disk_irq_enabled {
                self.disk_irq_pending = true;
            }        
        }
    }

    fn handle_write_mode_byte(&mut self) {
        // TODO!
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
        self.clock_timer_irq();
        self.update_disk_motor();
    }

    fn irq_flag(&self) -> bool {
        return self.timer_pending || self.disk_irq_pending;
    }

    fn read_cpu(&mut self, address: u16) -> Option<u8> {
        let data = match address {
            0x4030 => {
                let mut data = 0x00;
                if self.timer_pending {
                    data |= 0b0000_0001;
                }
                self.timer_pending = false;
                if self.byte_transfer_flag {
                    data |= 0b0000_0010;
                }
                self.byte_transfer_flag = false;
                
                if self.checksum != 0 {
                    data |= 0b0001_0000;
                }
                
                if self.rewinding {
                    data |= 0b0100_0000;
                }
                if !self.rewinding {
                    data |= 0b1000_0000;   
                }

                self.disk_irq_pending = false;
                Some(data)
            },
            0x4031 => {
                self.byte_transfer_flag = false;
                self.disk_irq_pending = false;
                Some(self.read_buffer)
            },
            0x4032 => {
                // We always have a disk in the drive, so for now leave this at 0
                let mut data = 0b0000_0000;
                if !self.disk_ready_flag {
                    data |= 0b0000_0010;
                }
                // should we set bit 6 here? I think it's technically open bus
                Some(data)
            }
            _ => {self.debug_read_cpu(address)}
        };
        return data;
    }
    
    fn debug_read_cpu(&self, address: u16) -> Option<u8> {
        match address {
            0x4033 => {
                // high bit set == battery good
                return Some(0x80 & self.expansion_port_buffer);
            },
            0x6000 ..= 0xDFFF => {Some(self.prg_ram[address as usize - 0x6000])},
            0xE000 ..= 0xFFFF => {Some(self.bios_rom[address as usize - 0xE000])},
            _ => None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
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
            0x4024 => {
                self.write_buffer = data;
                self.byte_transfer_flag = false;
                self.disk_irq_pending = false;
            },
            0x4025 => {
                let motor_disabled = (data & 0b0000_0001) == 0;
                let motor_on = (data & 0b0000_0010) == 0;
                if !motor_disabled && motor_on {
                    self.motor_on = true;
                } else {
                    self.motor_on = false;
                    self.disk_ready_flag = false;
                }
                self.write_mode = (data & 0b0000_0100) == 0;
                self.mirroring = match (data & 0b0000_1000) == 0 {
                    true => Mirroring::Vertical,
                    false => Mirroring::Horizontal,
                };
                // TODO: CRC control bit

                if (self.old_4025 & 0b0100_0000) == 0 {
                    self.transfer_reset_flag = (data & 0b0100_0000) != 0;
                }

                self.disk_irq_enabled = (data & 0b1000_0000) != 0;
                self.disk_irq_pending = false;

                self.old_4025 = data;
            },
            0x4026 => {
                self.expansion_port_buffer = data;
            }
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

pub fn expand_disk_image(compact_disk_image: &Vec<u8>) -> Vec<u8> {
    const BLOCK_1_SIZE: usize = 0x38;
    const BLOCK_2_SIZE: usize = 0x02;
    const FILE_HEADER_SIZE: usize = 0x10;
    const FILE_SIZE_OFFSET: usize = 0x0D;

    const LEADING_ZEROES: usize = 3537; // about 28300 bits
    const GAP_SIZE: usize = 122; // about 976 bits
    const FINAL_SIZE: usize = 81920; // a total guess! (~80k)

    println!("=== Beginning quickdisk expansion... ===");

    let block_one = &compact_disk_image[0 .. BLOCK_1_SIZE];
    let block_two = &compact_disk_image[BLOCK_1_SIZE .. BLOCK_1_SIZE + BLOCK_2_SIZE];

    let data_file_start = BLOCK_1_SIZE + BLOCK_2_SIZE;

    let mut expanded_image: Vec<u8> = Vec::new();
    let leading_zeroes = vec![0u8; LEADING_ZEROES - 1];
    let gap = vec![0u8; GAP_SIZE - 1]; // is this syntax valid? lol
    let fake_checksum = vec![77u8, 98u8];

    expanded_image.extend(leading_zeroes);
    expanded_image.push(0x80); // signals the start of a data block affects the checksum
    expanded_image.extend_from_slice(&block_one);
    expanded_image.extend(fake_checksum.clone());
    expanded_image.extend(gap.clone());
    expanded_image.push(0x80);
    expanded_image.extend_from_slice(&block_two);
    expanded_image.extend(fake_checksum.clone());

    let mut pos = data_file_start;
    while compact_disk_image[pos] == 0x03 {
        let file_header = &compact_disk_image[pos .. pos + FILE_HEADER_SIZE];
        pos += FILE_HEADER_SIZE;
        let file_size = (file_header[FILE_SIZE_OFFSET] as usize) | ((file_header[FILE_SIZE_OFFSET + 1] as usize) << 8);
        let file_block = &compact_disk_image[pos .. pos + file_size + 1];
        pos += file_size + 1;

        expanded_image.extend(gap.clone());
        expanded_image.push(0x80);
        expanded_image.extend_from_slice(&file_header);
        expanded_image.extend(fake_checksum.clone());

        expanded_image.extend(gap.clone());
        expanded_image.push(0x80);
        expanded_image.extend_from_slice(&file_block);
        expanded_image.extend(fake_checksum.clone());
    }

    expanded_image.resize(FINAL_SIZE, 0);
    return expanded_image;
}