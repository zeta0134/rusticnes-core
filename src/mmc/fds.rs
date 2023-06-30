// A very simple Mapper with no esoteric features or bank switching.
// Reference capabilities: https://wiki.nesdev.com/w/index.php/NROM

use fds::FdsFile;

use mmc::mapper::*;
use mmc::mirroring;

use apu::AudioChannelState;
use apu::PlaybackRate;
use apu::Volume;
use apu::Timbre;
use apu::RingBuffer;
use apu::filters;
use apu::filters::DspFilter;

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

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

    write_buffer: u8,
    read_buffer: u8,
    expansion_port_buffer: u8,

    disk_images: Vec<Vec<u8>>,
    current_side: usize,
    desired_side: usize,
    disk_change_cooldown: u32,

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
    transfer_active_flag: bool,
    checksum: u16,
    crc_control: bool,

    old_4025: u8,

    debug_old_cpuread: u16,
    debug_mode: bool,

    audio: FdsAudio,
}

impl FdsMapper {
    pub fn from_fds(fds: FdsFile) -> Result<FdsMapper, String> {
        // FOR NOW, use just the first disk and ignore the rest
        let mut expanded_disks = Vec::new();
        for i in 0 .. fds.disk_sides.len() {
            expanded_disks.push(expand_disk_image(&fds.disk_sides[i]));
        }

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

            write_buffer: 0,
            read_buffer: 0,
            expansion_port_buffer: 0,

            disk_images: expanded_disks,
            current_side: 0,
            desired_side: 0,
            disk_change_cooldown: 0,

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
            transfer_active_flag: false,
            checksum: 0,
            crc_control: false,

            old_4025: 0,

            debug_old_cpuread: 0,
            debug_mode: false,

            audio: FdsAudio::new(),
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

    fn update_disk_sides(&mut self) {
        if self.desired_side != self.current_side {
            self.disk_change_cooldown = 1000000; // CPU cycles before the disk becomes available again
            println!("Ejected disk #{}", self.current_side);
            self.current_side = self.desired_side;
        }
        if self.disk_change_cooldown > 0 {
            self.disk_change_cooldown -= 1;
            if self.disk_change_cooldown == 0 {
                println!("Inserted disk {}", self.current_side);
            }
        }
    }

    fn update_disk_motor(&mut self) {
        if self.disk_change_cooldown > 0 {
            // Presumably the motor at least pauses when a disk is ejected
            return;
        }

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
        let current_data_byte = self.disk_images[self.current_side][self.head_position];

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
        // TODO: if CRC control is set, we need to write the computed CRC out here
        if self.transfer_active_flag {
            if self.crc_control {
                let crc_byte = (self.checksum & 0x00FF) as u8;
                self.checksum = self.checksum >> 8;
                self.disk_images[self.current_side][self.head_position] = crc_byte;
            } else {
                self.disk_images[self.current_side][self.head_position] = self.write_buffer;
            }
            self.byte_transfer_flag = true;
            if self.disk_irq_enabled {
                self.disk_irq_pending = true;
            }
        } else {
            self.disk_images[self.current_side][self.head_position] = 0x00;
        }
    }

    fn snoop_bios_calls(&mut self, address: u16) {
        // Only consider this execution if we got the full opcode preamble
        if address == self.debug_old_cpuread + 1 {
            // The first address in the pair should match the 
            match self.debug_old_cpuread {
                0xE1F8 => println!("=== BIOS: LoadFiles ==="),
                0xE237 => println!("=== BIOS: AppendFile ==="),
                0xE239 => println!("=== BIOS: WriteFile ==="),
                0xE2B7 => println!("=== BIOS: CheckFileCount ==="),
                0xE2BB => println!("=== BIOS: AdjustFileCount ==="),
                0xE301 => println!("=== BIOS: SetFileCount1 ==="),
                0xE305 => println!("=== BIOS: SetFileCount ==="),
                0xE32A => println!("=== BIOS: GetDiskInfo ==="),

                0xE445 => println!("=== BIOS: CheckDiskHeader ==="),
                0xE484 => println!("=== BIOS: GetNumFiles ==="),
                0xE492 => println!("=== BIOS: SetNumFiles ==="),
                0xE4A0 => println!("=== BIOS: FileMatchTest ==="),
                0xE4DA => println!("=== BIOS: SkipFiles ==="),
                _ => {}
            }
        }
        self.debug_old_cpuread = address;
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
        self.update_disk_sides();
        self.update_disk_motor();
        self.audio.clock_cpu();
    }

    fn mix_expansion_audio(&self, nes_sample: f32) -> f32 {
        let fds_sample = self.audio.output();
        
        // The maximum volume of the FDS signal on a Famicom is roughly 2.4x the maximum volume of the APU square
        let nes_pulse_full_volume = 95.88 / ((8128.0 / 15.0) + 100.0);
        let fds_weight = nes_pulse_full_volume * 2.4;

        return 
            (fds_sample * fds_weight) + 
            nes_sample;
    }

    fn irq_flag(&self) -> bool {
        return self.timer_pending || self.disk_irq_pending;
    }

    fn read_cpu(&mut self, address: u16) -> Option<u8> {
        if self.debug_mode {
            self.snoop_bios_calls(address);
        }
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
                // Disk inserted (1 == ejected)
                if self.disk_change_cooldown > 0 {
                    data |= 0b0000_0001;
                }
                // Transfer ready flag (0 == ready)
                if (self.disk_change_cooldown > 0) || (!self.disk_ready_flag) {
                    data |= 0b0000_0010;
                }
                // Writable (1 == read-only or ejected) (all emulated disks are r/w)
                if self.disk_change_cooldown > 0 {
                    data |= 0b0000_0100;
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
                self.crc_control = (data & 0b0001_0000) != 0;
                if self.crc_control {
                    self.checksum = 0x624D;
                }

                self.transfer_active_flag = (data & 0b0100_0000) != 0;
                if (self.old_4025 & 0b0100_0000) == 0 {
                    self.transfer_reset_flag = self.transfer_active_flag;
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
        self.audio.write_cpu(address, data);
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

    fn switch_disk(&mut self, side: usize) {
        if side <= self.disk_images.len()  {
            self.desired_side = side;
        } else {
            println!("No disk with side {} present, refusing to switch.", side);
        }
    }

    fn has_sram(&self) -> bool {
        // There is no header flag to tell us otherwise, so we assume all disks are writeable and therefore saveable
        return true;
    }

    fn get_sram(&self) -> Vec<u8> {
        let mut combined_disk_images = Vec::new();
        for i in 0 .. self.disk_images.len() {
            combined_disk_images.extend(self.disk_images[i].clone());
        }
        return combined_disk_images;
    }

    fn load_sram(&mut self, raw_data: Vec<u8>) {
        if raw_data.len() != self.disk_images.len() * 81920 {
            println!("Wrong .sav file size for currently loaded FDS image! Refusing to load.");
            return;
        }

        let mut expanded_disk_images = Vec::new();
        for i in 0 .. self.disk_images.len() {
            let start = i * 81920;
            let end = start + 81920;
            let mut disk = Vec::new();
            disk.extend_from_slice(&raw_data[start .. end]);
            expanded_disk_images.push(disk);
        }

        self.disk_images = expanded_disk_images;
    }

    fn channels(&self) ->  Vec<& dyn AudioChannelState> {
        let mut channels: Vec<& dyn AudioChannelState> = Vec::new();
        channels.push(&self.audio);
        return channels;
    }

    fn channels_mut(&mut self) ->  Vec<&mut dyn AudioChannelState> {
        let mut channels: Vec<&mut dyn AudioChannelState> = Vec::new();
        channels.push(&mut self.audio);
        return channels;
    }

    fn record_expansion_audio_output(&mut self, _nes_sample: f32) {
        self.audio.record_current_output();
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


pub struct FdsAudio {
    enable_sound_registers: bool,
    wavetable_ram: [u8; 64],

    volume_envelope_output: u8,
    volume_envelope_value: u8,
    volume_envelope_positive: bool,
    volume_envelope_disabled: bool,

    volume_envelope_counter_current: usize,
    volume_envelope_counter_initial: usize,

    frequency: usize,
    frequency_envelope_disable: bool,
    frequency_halt: bool,

    frequency_accumulator: usize,

    mod_envelope_output: u8,
    mod_envelope_value: u8,
    mod_envelope_positive: bool,
    mod_envelope_disabled: bool,

    mod_accumulator: usize,

    mod_envelope_counter_current: usize,
    mod_envelope_counter_initial: usize,

    mod_counter: i8,

    mod_frequency: usize,
    mod_always_carry: bool,
    mod_table_halt: bool,

    mod_table: [u8; 32],
    master_volume: u8,
    wave_write_enabled: bool,

    master_envelope_speed: u8,

    mod_position: usize,
    wave_position: usize,
    
    current_output: f32,

    debug_disable: bool,
    output_buffer: RingBuffer,
    edge_buffer: RingBuffer,
    last_edge: bool,
    debug_filter: filters::HighPassIIR,
}

impl FdsAudio {
    pub fn new() -> FdsAudio {
        return FdsAudio {
            enable_sound_registers: true,
            wavetable_ram: [0u8; 64],

            volume_envelope_output: 0,
            volume_envelope_value: 0,
            volume_envelope_positive: false,
            volume_envelope_disabled: true,

            volume_envelope_counter_current: 0,
            volume_envelope_counter_initial: 0,

            frequency: 0,
            frequency_envelope_disable: false,
            frequency_halt: false,

            frequency_accumulator: 0,

            mod_envelope_output: 0,
            mod_envelope_value: 0,
            mod_envelope_positive: false,
            mod_envelope_disabled: true,

            mod_accumulator: 0,

            mod_envelope_counter_current: 0,
            mod_envelope_counter_initial: 0,

            mod_counter: 0,

            mod_frequency: 0,
            mod_always_carry: false,
            mod_table_halt: false,

            mod_table: [0u8; 32],
            master_volume: 0,
            wave_write_enabled: true,

            master_envelope_speed: 0xE8,

            mod_position: 0,
            wave_position: 0,

            current_output: 0.0,

            debug_disable: false,
            output_buffer: RingBuffer::new(32768),
            edge_buffer: RingBuffer::new(32768),
            last_edge: false,
            debug_filter: filters::HighPassIIR::new(44100.0, 300.0),
        }
    }

    pub fn envelope_ticks(&self, envelope_value: u8) -> usize {
        let base_rate = if self.frequency_halt {2} else {8};
        return base_rate * (envelope_value as usize + 1) * (self.master_envelope_speed as usize + 1);
    }

    pub fn tick_volume_envelope(&mut self) {
        if self.volume_envelope_disabled {
            self.volume_envelope_output = self.volume_envelope_value;
        } else {
            if self.volume_envelope_counter_current == 0 {
                if self.volume_envelope_positive && self.volume_envelope_output < 32 {
                    self.volume_envelope_output += 1;
                } else if (!self.volume_envelope_positive) && (self.volume_envelope_output > 0) {
                    self.volume_envelope_output -= 1;
                }
                self.volume_envelope_counter_current = self.volume_envelope_counter_initial;
            } else {
                self.volume_envelope_counter_current -= 1;
            }
        }
    }

    pub fn tick_mod_envelope(&mut self) {
        if self.mod_envelope_disabled {
            self.mod_envelope_output = self.mod_envelope_value;
        } else {
            if self.mod_envelope_counter_current == 0 {
                if self.mod_envelope_positive && self.mod_envelope_output < 63 {
                    self.mod_envelope_output += 1;
                } else if (!self.mod_envelope_positive) && (self.mod_envelope_output > 0) {
                    self.mod_envelope_output -= 1;
                }
                self.mod_envelope_counter_current = self.mod_envelope_counter_initial;
            } else {
                self.mod_envelope_counter_current -= 1;
            }
        }
    }

    pub fn tick_mod_unit(&mut self) {
        if self.mod_table_halt {
            return; // do nothing!
        }

        let shifted_mod_position = self.mod_position >> 1;
        let mod_behavior_index = self.mod_table[shifted_mod_position];
        self.mod_position = (self.mod_position + 1) & 63;
        // Note: the mod counter is a signed 7-bit value. Here we simulate this behavior
        // by doubling all of the modifications we would make to it, and then shifting the
        // result to put it in the proper range.
        match mod_behavior_index {
            0b000 => {},
            0b001 => {self.mod_counter += 2},
            0b010 => {self.mod_counter += 4},
            0b011 => {self.mod_counter += 8},
            0b100 => {self.mod_counter  = 0},
            0b101 => {self.mod_counter -= 8},
            0b110 => {self.mod_counter -= 4},
            0b111 => {self.mod_counter -= 2},
            _ => {} // shouldn't be reachable
        }
    }

    pub fn mod_pitch(&self) -> i32 {
        let shifted_counter = self.mod_counter >> 1;

        // 1. multiply counter by gain, lose lowest 4 bits of result but "round" in a strange way
        let mut temp = shifted_counter as i32 * (self.mod_envelope_output as i32);
        let mut remainder = temp & 0xF;
        temp = temp >> 4;
        if (remainder > 0) && ((temp & 0x80) == 0) {
            if shifted_counter < 0 {
                temp -= 1;
            } else {
                temp += 2;
            }
        }

        // 2. wrap if a certain range is exceeded
        if temp >= 192 {
            temp -= 256;
        } else if temp < -64 {
            temp += 256;
        }

        // 3. multiply result by pitch, then round to nearest while dropping 6 bits
        temp = self.frequency as i32 * temp;
        remainder = temp & 0x3F;
        temp = temp >> 6;
        if remainder >= 32 {
            temp += 1;
        }

        return temp;
    }

    pub fn tick_wave_unit(&mut self) {
        self.wave_position = (self.wave_position + 1) & 63;
        if self.wave_position == 0 {
            self.last_edge = true;
        }
    }

    pub fn update_mod(&mut self) {
        self.mod_accumulator += self.mod_frequency;
        //if self.mod_accumulator >= 4096 {
        //    self.mod_accumulator -= 4096;
        if self.mod_accumulator >= 65536 {
            self.mod_accumulator -= 65536;
            self.tick_mod_unit();
        } else if self.mod_always_carry {
            self.tick_mod_unit();
        }
    }

    pub fn update_wave(&mut self) {
        if self.frequency_halt {
            return;
        }
        self.frequency_accumulator += std::cmp::max((self.frequency as i32) + self.mod_pitch(), 0) as usize;
        if self.frequency_accumulator >= 65536 {
            self.frequency_accumulator -= 65536;
            self.tick_wave_unit();
        }
    }

    pub fn clock_cpu(&mut self) {
        if self.frequency_envelope_disable {
            self.volume_envelope_counter_current = self.volume_envelope_counter_initial;
            self.mod_envelope_counter_current = self.mod_envelope_counter_initial;
        } else {
            self.tick_volume_envelope();
            self.tick_mod_envelope();
        }
        self.update_mod();
        self.update_wave();
        self.compute_output();
    }

    pub fn compute_output(&mut self) {
        if !self.wave_write_enabled {
            let current_sample = self.wavetable_ram[self.wave_position] as f32 / 63.0;
            let volume_attenuated_sample = (current_sample * std::cmp::min(self.volume_envelope_output, 32) as f32) / 32.0;
            let master_attenuated_sample = match self.master_volume {
                0 => volume_attenuated_sample,
                1 => volume_attenuated_sample * 2.0 / 3.0,
                2 => volume_attenuated_sample * 2.0 / 4.0,
                3 => volume_attenuated_sample * 2.0 / 5.0,
                _ => {0.0} // unreachable
            };
            self.current_output = master_attenuated_sample;
        }
    }

    pub fn output(&self) -> f32 {
        return self.current_output;
    }

    pub fn write_cpu(&mut self, address: u16, data: u8) {
        match address {
            0x4023 => {
                self.enable_sound_registers = (data & 0b0000_0010) != 0;
            },
            0x4040 ..= 0x407F => {
                if self.wave_write_enabled {
                    let wave_pos = (address - 0x4040) as usize;
                    self.wavetable_ram[wave_pos] = data & 63;
                }
            },
            0x4080 => {
                self.volume_envelope_disabled = (data & 0b1000_0000) != 0;
                self.volume_envelope_positive = (data & 0b0100_0000) != 0;
                self.volume_envelope_value = data & 0b0011_1111;
                self.volume_envelope_counter_initial = self.envelope_ticks(self.volume_envelope_value);
                self.volume_envelope_counter_current = self.volume_envelope_counter_initial;
            },
            0x4082 => {
                self.frequency = (self.frequency & 0xFF00) | (data as usize);
            },
            0x4083 => {
                self.frequency = (self.frequency & 0x00FF) | (((data & 0b0000_1111) as usize) << 8);
                self.frequency_envelope_disable = (data & 0b0100_0000) != 0;
                self.frequency_halt = (data & 0b1000_0000) != 0;
                if self.frequency_halt {
                    self.wave_position = 0;
                }
            },
            0x4084 => {
                self.mod_envelope_disabled = (data & 0b1000_0000) != 0;
                self.mod_envelope_positive = (data & 0b0100_0000) != 0;
                self.mod_envelope_value = data & 0b0011_1111;
                self.mod_envelope_counter_initial = self.envelope_ticks(self.mod_envelope_value);
                self.mod_envelope_counter_current = self.mod_envelope_counter_initial;
            },
            0x4085 => {
                self.mod_counter = ((data & 0b0111_1111) << 1) as i8;
            },
            0x4086 => {
                self.mod_frequency = (self.mod_frequency & 0xFF00) | (data as usize);
            },
            0x4087 => {
                self.mod_frequency = (self.mod_frequency & 0x00FF) | (((data & 0b0000_1111) as usize) << 8);
                self.mod_always_carry = (data & 0b0100_0000) != 0;
                self.mod_table_halt = (data & 0b1000_0000) != 0;
                if self.mod_table_halt {
                    self.mod_accumulator = 0;
                }
            },
            0x4088 => {
                self.mod_table[self.mod_position >> 1] = data & 0b0000_0111;
                self.mod_position = (self.mod_position + 2) & 63;
            },
            0x4089 => {
                self.master_volume = data & 0b0000_0011;
                self.wave_write_enabled = (data & 0b1000_0000) != 0;
            },
            0x408A => {
                self.master_envelope_speed = data;
            },
            _ => {}
        }
    }
}

impl AudioChannelState for FdsAudio {
    fn name(&self) -> String {
        return "Wavetable".to_string();
    }

    fn chip(&self) -> String {
        return "FDS".to_string();
    }

    fn sample_buffer(&self) -> &RingBuffer {
        return &self.output_buffer;
    }

    fn edge_buffer(&self) -> &RingBuffer {
        return &self.edge_buffer;
    }

    fn record_current_output(&mut self) {
        self.debug_filter.consume(self.output() as f32);
        self.output_buffer.push((self.debug_filter.output() * -4096.0) as i16);
        self.edge_buffer.push(self.last_edge as i16);
        self.last_edge = false;
    }

    fn min_sample(&self) -> i16 {
        return -4096;
    }

    fn max_sample(&self) -> i16 {
        return 4096;
    }

    fn muted(&self) -> bool {
        return self.debug_disable;
    }

    fn mute(&mut self) {
        self.debug_disable = true;
    }

    fn unmute(&mut self) {
        self.debug_disable = false;
    }

    fn playing(&self) -> bool {
        return 
            !self.frequency_halt &&
            self.frequency > 0 &&
            self.volume_envelope_output > 0;
    }

    fn rate(&self) -> PlaybackRate {
        let p = (self.frequency as i32 + self.mod_pitch()) as f32;
        let n = 1789773.0;
        let f = (n * p / 65536.0) / 64.0;
        return PlaybackRate::FundamentalFrequency {frequency: f};
    }

    fn volume(&self) -> Option<Volume> {
        let env_volume = std::cmp::min(self.volume_envelope_output, 32) as f32;
        let effective_volume = match self.master_volume {
            0 => env_volume * 100.0,
            1 => env_volume * 200.0 / 3.0,
            2 => env_volume * 200.0 / 4.0,
            3 => env_volume * 200.0 / 5.0,
            _ => {0.0} // unreachable
        };
        return Some(Volume::VolumeIndex{ index: effective_volume as usize, max: 3200 });
    }

    fn timbre(&self) -> Option<Timbre> {
        let mut hasher = DefaultHasher::new();
        let audio_data = &self.wavetable_ram[0 .. 64];
        hasher.write(audio_data);
        let full_result = hasher.finish();
        let truncated_result = (full_result & 0xFF) as usize;

        return Some(Timbre::PatchIndex{ index: truncated_result, max: 255 });
    }
}