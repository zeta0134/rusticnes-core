// Sunsoft FME-7, 5A, and 5B (notably lacking expansion audio for now)
// Reference implementation: https://wiki.nesdev.com/w/index.php/Sunsoft_FME-7

use cartridge::NesHeader;
use mmc::mapper::*;
use mmc::mirroring;

use apu::AudioChannelState;
use apu::PlaybackRate;
use apu::Volume;
use apu::Timbre;
use apu::RingBuffer;

pub struct Fme7 {
    pub prg_rom: Vec<u8>,
    pub prg_ram: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub command: u8,
    pub chr_banks: Vec<usize>,
    pub prg_banks: Vec<usize>,
    pub prg_ram_enabled: bool,
    pub prg_ram_selected: bool,
    pub vram: Vec<u8>,
    pub mirroring: Mirroring,
    pub irq_enabled: bool,
    pub irq_counter_enabled: bool,
    pub irq_counter: u16,
    pub irq_pending: bool,
    pub audio_command_select: u8,
    expansion_audio_chip: YM2149F,
}

impl Fme7 {
    pub fn new(header: NesHeader, chr: &[u8], prg: &[u8]) -> Fme7 {
        return Fme7 {
            prg_rom: prg.to_vec(),
            chr_rom: chr.to_vec(),
            prg_ram: vec![0u8; header.prg_ram_size],
            command: 0,
            chr_banks: vec![0usize; 8],
            prg_banks: vec![0usize; 4],
            prg_ram_enabled: false,
            prg_ram_selected: false,
            vram: vec![0u8; 0x1000],
            mirroring: Mirroring::Vertical,
            irq_enabled: false,
            irq_counter_enabled: false,
            irq_counter: 0,
            irq_pending: false,
            audio_command_select: 0,
            expansion_audio_chip: YM2149F::new(),
        }
    }

    pub fn clock_irq(&mut self) {
        if self.irq_counter_enabled {
            self.irq_counter = self.irq_counter.wrapping_sub(1);
            if self.irq_counter == 0xFFFF {
                self.irq_pending = true;
            }
        }
    }
}

impl Mapper for Fme7 {
    fn mirroring(&self) -> Mirroring {
        return Mirroring::Horizontal;
    }
    
    fn debug_read_cpu(&self, address: u16) -> Option<u8> {
        let prg_rom_len = self.prg_rom.len();
        let prg_ram_len = self.prg_ram.len();
        match address {
            0x6000 ..= 0x7FFF => {
                if self.prg_ram_selected {
                    if self.prg_ram_enabled {
                        return Some(self.prg_ram[((self.prg_banks[0] * 0x2000) + (address as usize - 0x6000)) % prg_ram_len]);
                    } else {
                        return None
                    }
                } else {
                    return Some(self.prg_rom[((self.prg_banks[0] * 0x2000) + (address as usize - 0x6000)) % prg_rom_len]);
                }
            },
            0x8000 ..= 0x9FFF => return Some(self.prg_rom[((self.prg_banks[1] * 0x2000) + (address as usize - 0x8000)) % prg_rom_len]),
            0xA000 ..= 0xBFFF => return Some(self.prg_rom[((self.prg_banks[2] * 0x2000) + (address as usize - 0xA000)) % prg_rom_len]),
            0xC000 ..= 0xDFFF => return Some(self.prg_rom[((self.prg_banks[3] * 0x2000) + (address as usize - 0xC000)) % prg_rom_len]),
            0xE000 ..= 0xFFFF => return Some(self.prg_rom[(prg_rom_len - 0x2000) + (address as usize - 0xE000)]),
            _ => return None
        }
    }

    fn clock_cpu(&mut self) {
        self.clock_irq();
        self.expansion_audio_chip.clock();
    }

    fn debug_read_ppu(&self, address: u16) -> Option<u8> {
        let chr_rom_len = self.chr_rom.len();
        match address {
            0x0000 ..= 0x03FF => return Some(self.chr_rom[((self.chr_banks[0] * 0x400) + (address as usize - 0x0000)) % chr_rom_len]),
            0x0400 ..= 0x07FF => return Some(self.chr_rom[((self.chr_banks[1] * 0x400) + (address as usize - 0x0400)) % chr_rom_len]),
            0x0800 ..= 0x0BFF => return Some(self.chr_rom[((self.chr_banks[2] * 0x400) + (address as usize - 0x0800)) % chr_rom_len]),
            0x0C00 ..= 0x0FFF => return Some(self.chr_rom[((self.chr_banks[3] * 0x400) + (address as usize - 0x0C00)) % chr_rom_len]),
            0x1000 ..= 0x13FF => return Some(self.chr_rom[((self.chr_banks[4] * 0x400) + (address as usize - 0x1000)) % chr_rom_len]),
            0x1400 ..= 0x17FF => return Some(self.chr_rom[((self.chr_banks[5] * 0x400) + (address as usize - 0x1400)) % chr_rom_len]),
            0x1800 ..= 0x1BFF => return Some(self.chr_rom[((self.chr_banks[6] * 0x400) + (address as usize - 0x1800)) % chr_rom_len]),
            0x1C00 ..= 0x1FFF => return Some(self.chr_rom[((self.chr_banks[7] * 0x400) + (address as usize - 0x1C00)) % chr_rom_len]),
            0x2000 ..= 0x3FFF => return match self.mirroring {
                Mirroring::Horizontal => Some(self.vram[mirroring::horizontal_mirroring(address) as usize]),
                Mirroring::Vertical   => Some(self.vram[mirroring::vertical_mirroring(address) as usize]),
                Mirroring::OneScreenLower => Some(self.vram[mirroring::one_screen_lower(address) as usize]),
                Mirroring::OneScreenUpper => Some(self.vram[mirroring::one_screen_upper(address) as usize]),
                _ => None
            },
            _ => return None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        //self.clock_irq();
        match address {
            0x6000 ..= 0x7FFF => {
                if self.prg_ram_selected && self.prg_ram_enabled {
                    let prg_ram_len = self.prg_ram.len();
                    self.prg_ram[((self.prg_banks[0] * 0x2000) + (address as usize - 0x6000)) % prg_ram_len] = data;
                }
            },
            0x8000 ..= 0x9FFF => {
                // Store the command to execute next
                self.command = data & 0b0000_1111;
            },
            0xA000 ..= 0xBFFF => {
                // Execute the stored command with the provided parameter byte
                match self.command {
                    0x0 ..= 0x7 => { 
                        self.chr_banks[self.command as usize] = data as usize
                    },
                    0x8 =>  {
                        self.prg_ram_enabled = (data & 0b1000_0000) != 0;
                        self.prg_ram_selected = (data & 0b0100_0000) != 0;
                        self.prg_banks[0] = (data & 0b0011_1111) as usize;
                    },
                    0x9 ..= 0xB => {
                        self.prg_banks[(self.command - 0x8) as usize] = (data & 0b0011_1111) as usize;
                    },
                    0xC => {
                        match data & 0b0000_0011 {
                            0 => self.mirroring = Mirroring::Vertical,
                            1 => self.mirroring = Mirroring::Horizontal,
                            2 => self.mirroring = Mirroring::OneScreenLower,
                            3 => self.mirroring = Mirroring::OneScreenUpper,
                            _ => {}
                        }
                    },
                    0xD => {
                        // writes to this register always acknowledge any pending IRQ
                        self.irq_pending = false;
                        self.irq_enabled = (data & 0b0000_0001) != 0;
                        self.irq_counter_enabled = (data & 0b1000_0000) != 0;
                    },
                    0xE => {
                        self.irq_counter = (self.irq_counter & 0xFF00) + (data as u16);
                    },
                    0xF => {
                        self.irq_counter = (self.irq_counter & 0x00FF) + ((data as u16) << 8);
                    },
                    _ => {}
                }
            },
            0xC000 ..= 0xDFFF => {
                self.audio_command_select = data & 0x0F;
            },
            0xE000 ..= 0xFFFF => {
                self.expansion_audio_chip.execute_command(self.audio_command_select, data);
            }

            _ => {}
        }
    }

    fn write_ppu(&mut self, address: u16, data: u8) {
        match address {
            0x2000 ..= 0x3FFF => match self.mirroring {
                Mirroring::Horizontal => self.vram[mirroring::horizontal_mirroring(address) as usize] = data,
                Mirroring::Vertical   => self.vram[mirroring::vertical_mirroring(address) as usize] = data,
                Mirroring::OneScreenLower => self.vram[mirroring::one_screen_lower(address) as usize] = data,
                Mirroring::OneScreenUpper => self.vram[mirroring::one_screen_upper(address) as usize] = data,
                _ => {}
            },
            _ => {}
        }
    }

    fn irq_flag(&self) -> bool {
        return self.irq_enabled && self.irq_pending;
    }

    fn mix_expansion_audio(&self, nes_sample: f64) -> f64 {
        return (self.expansion_audio_chip.output() - 0.5) * 1.06 - nes_sample;
    }

    fn channels(&self) ->  Vec<& dyn AudioChannelState> {
        let mut channels: Vec<& dyn AudioChannelState> = Vec::new();
        channels.push(&self.expansion_audio_chip.channel_a);
        channels.push(&self.expansion_audio_chip.channel_b);
        channels.push(&self.expansion_audio_chip.channel_c);
        return channels;
    }

    fn channels_mut(&mut self) ->  Vec<&mut dyn AudioChannelState> {
        let mut channels: Vec<&mut dyn AudioChannelState> = Vec::new();
        channels.push(&mut self.expansion_audio_chip.channel_a);
        channels.push(&mut self.expansion_audio_chip.channel_b);
        channels.push(&mut self.expansion_audio_chip.channel_c);
        return channels;
    }

    fn record_expansion_audio_output(&mut self) {
        self.expansion_audio_chip.record_output();
    }
}

struct ToneGenerator {
    pub period_compare: u16,
    pub period_current: u16,
    pub output: u8,
}

impl ToneGenerator {
    pub fn new() -> ToneGenerator {
        return ToneGenerator {
            period_compare: 0,
            period_current: 0,
            output: 0,
        }
    }

    pub fn clock(&mut self) {
        self.period_current += 1;
        if self.period_current >= self.period_compare {
            self.period_current = 0;
            self.output = self.output ^ 0b1;
        }
    }

    pub fn output(&self) -> u8 {
        return self.output;
    }
}

struct NoiseGenerator {
    pub period_compare: u16,
    pub period_current: u16,
    // Actually a 17bit register, higher bits are unused
    pub shift_register: u32,
}

impl NoiseGenerator {
    pub fn new() -> NoiseGenerator {
        return NoiseGenerator {
            period_compare: 0,
            period_current: 0,
            shift_register: 0b1_1111_1111_1111_1111,
        }
    }

    pub fn advance_lfsr(&mut self) {
        let tap16 = (self.shift_register & 0b0000_0000_0000_0000_0010) >> 1;
        let tap13 = (self.shift_register & 0b0000_0000_0000_0001_0000) >> 4;
        let new_bit_16 = tap13 ^ tap16;
        self.shift_register = self.shift_register >> 1;
        self.shift_register += new_bit_16 << 16
    }

    pub fn clock(&mut self) {
        self.period_current += 1;
        if self.period_current >= self.period_compare {
            self.period_current = 0;
            self.advance_lfsr();
        }
    }

    pub fn output(&self) -> u8 {
        return (self.shift_register & 0b1) as u8;
    }
}

struct EnvelopeGenerator {
    pub period_compare: u16,
    pub period_current: u16,
    pub continue_flag: bool,
    pub attack_flag: bool,
    pub alternate_flag: bool,
    pub hold_flag: bool,
    pub current_value: i8,
    pub increasing: bool,
    pub holding: bool,
}

impl EnvelopeGenerator {
    pub fn new() -> EnvelopeGenerator {
        return EnvelopeGenerator {
            period_compare: 0,
            period_current: 0,
            continue_flag: false,
            attack_flag: false,
            alternate_flag: false,
            hold_flag: false,
            current_value: 0,
            increasing: false,
            holding: false,
        }
    }

    pub fn restart_envelope(&mut self) {
        self.holding = false;
        if self.attack_flag {
            self.increasing = true;
            self.current_value = 0;
        } else {
            self.increasing = false;
            self.current_value = 31;
        }
    }

    pub fn advance_envelope(&mut self) {
        if self.holding {
            return;
        }

        if self.increasing {
            self.current_value += 1;
        } else {
            self.current_value -= 1;
        }

        if (self.current_value == -1) || (self.current_value == 32) {
            // We've reached a boundary; decide how to proceed
            if !(self.continue_flag) {
                // non-continue mode, choose a value to hold
                // and exit immediately. Note an oddity here,
                // we *always* hold the value 0 in non-continue
                // mode.
                self.current_value = 0;
                self.holding = true;
            } else {
                if self.hold_flag {
                    // Hold this value, with an optional flip first
                    // (this is the only way to get the more intuitive
                    // "increase and hold" behavior)
                    self.holding = true;
                    if self.alternate_flag {
                        if self.attack_flag {
                            self.current_value = 0;
                        } else {
                            self.current_value = 31;
                        }
                    }
                }

                // Deal with switching directions, and fix the 5-bit overflow
                if self.alternate_flag {
                    if self.increasing {
                        self.current_value = 31;
                    } else {
                        self.current_value = 0;
                    }
                    self.increasing = !(self.increasing);
                } else {
                    if self.increasing {
                        self.current_value = 0;
                    } else {
                        self.current_value = 31;
                    }
                }
            }
        }
    }

    pub fn clock(&mut self) {
        self.period_current += 1;
        if self.period_current >= self.period_compare {
            self.period_current = 0;
            self.advance_envelope();
        }
    }

    pub fn output(&self) -> usize {
        return self.current_value as usize;
    }
}

struct YmChannel {
    pub name: String,
    pub output_buffer: RingBuffer,
    pub muted: bool,

    pub tone: ToneGenerator,
    pub tone_enabled: bool,
    pub noise_enabled: bool,
    pub envelope_enabled: bool,
    pub static_volume: u8,
    pub effective_volume: usize,
    pub effective_amplitude: f64,
}

impl YmChannel {
    pub fn new(channel_name: &str) -> YmChannel {
        return YmChannel {
            name: String::from(channel_name),
            output_buffer: RingBuffer::new(32768),
            muted: false,
            tone: ToneGenerator::new(),
            tone_enabled: false,
            noise_enabled: false,
            envelope_enabled: false,
            static_volume: 0,
            effective_volume: 0,
            effective_amplitude: 0.0,
        }
    }

    pub fn record_sample(&mut self, sample: i16) {
        self.output_buffer.push(sample);
    }
}

impl AudioChannelState for YmChannel {
    fn name(&self) -> String {
        return self.name.clone();
    }

    fn chip(&self) -> String {
        return "YM2149F".to_string();
    }

    fn sample_buffer(&self) -> &RingBuffer {
        return &self.output_buffer;
    }

    fn record_current_output(&mut self) {
        // not used, we do this manually in YM2149F
    }

    fn min_sample(&self) -> i16 {
        return 0;
    }

    fn max_sample(&self) -> i16 {
        return 31;
    }

    fn muted(&self) -> bool {
        return self.muted;
    }

    fn mute(&mut self) {
        self.muted = true;
    }

    fn unmute(&mut self) {
        self.muted = false;
    }

    fn playing(&self) -> bool {
        return 
            !self.muted &&
            self.tone_enabled &&
            self.effective_volume > 1;
    }

    fn rate(&self) -> PlaybackRate {
        let frequency = 1_789_773.0 / (32.0 * (self.tone.period_compare as f64));
        return PlaybackRate::FundamentalFrequency {frequency: frequency};
    }

    fn volume(&self) -> Option<Volume> {
        return Some(Volume::VolumeIndex{ index: self.effective_volume, max: 31 });
    }

    fn timbre(&self) -> Option<Timbre> {
        return None;
    }

    fn amplitude(&self) -> f64 {
        if self.playing() {
            // Per: https://forums.nesdev.com/viewtopic.php?f=2&t=17745&sid=158b0a9e442a815411f7b453b093474a&start=15#p225103
            // "...its pre-compression output for a single channel only really goes up to maybe 8 db louder than an APU square can go"
            let db_boost = 10f64.powf(8.0 * 0.05);
            return self.effective_amplitude * db_boost;
        }
        return 0.0;
    }
}

struct YM2149F {
    pub channel_a: YmChannel,
    pub channel_b: YmChannel,
    pub channel_c: YmChannel,
    pub noise: NoiseGenerator,
    pub envelope: EnvelopeGenerator,
    pub clock_divider_counter: u8,
    pub volume_lut: Vec<f64>,
}

impl YM2149F {
    pub fn new() -> YM2149F {
        return YM2149F {
            channel_a: YmChannel::new("A"),
            channel_b: YmChannel::new("B"),
            channel_c: YmChannel::new("C"),
            noise: NoiseGenerator::new(),
            envelope: EnvelopeGenerator::new(),
            clock_divider_counter: 0,
            volume_lut: YM2149F::generate_volume_lut(),
        }
    }

    pub fn generate_volume_lut() -> Vec<f64> {
        let mut lut = vec![0f64; 32];
        lut[0] = 0.0;
        lut[1] = 0.0; // First two entries emit silence
        // The table should cap out at 1.0
        let mut output = 1.0;
        // Working our way down from the top...
        for i in (2 ..= 31).rev() {
            lut[i] = output;
            // ...decrease by 1.5 dB every step
            output /= 10f64.powf(1.5 * 0.05);
        }
        return lut;
    }

    pub fn effective_volume(&self, channel: &YmChannel) -> usize {
        let mut volume_index = (channel.static_volume as usize * 2) + 1;
        if channel.envelope_enabled {
            volume_index = self.envelope.output();
        }
        if volume_index > 1 {
            return volume_index;
        }
        return 0;
    }

    pub fn clock(&mut self) {
        self.clock_divider_counter += 1;
        if self.clock_divider_counter == 16 {
            self.envelope.clock();
            self.channel_a.tone.clock();
            self.channel_b.tone.clock();
            self.channel_c.tone.clock();
        }
        if self.clock_divider_counter == 32 {
            self.envelope.clock();
            self.channel_a.tone.clock();
            self.channel_b.tone.clock();
            self.channel_c.tone.clock();
            self.noise.clock();
            self.clock_divider_counter = 0;
        }
        self.channel_a.effective_volume = self.effective_volume(&self.channel_a);
        self.channel_b.effective_volume = self.effective_volume(&self.channel_b);
        self.channel_c.effective_volume = self.effective_volume(&self.channel_c);
        self.channel_a.effective_amplitude = self.volume_lut[self.channel_a.effective_volume];
        self.channel_b.effective_amplitude = self.volume_lut[self.channel_b.effective_volume];
        self.channel_c.effective_amplitude = self.volume_lut[self.channel_c.effective_volume];
    }

    pub fn channel_output(&self, channel: &YmChannel) -> usize {
        let mut signal_bit = 1u8;
        if channel.tone_enabled {
            signal_bit &= channel.tone.output();
        }
        if channel.noise_enabled {
            signal_bit &= self.noise.output();
        }
        if signal_bit != 0 && !channel.muted {
            return self.effective_volume(channel);
        }
        return 0;
    }

    pub fn output(&self) -> f64 {
        let channel_a = self.volume_lut[self.channel_output(&self.channel_a)];
        let channel_b = self.volume_lut[self.channel_output(&self.channel_b)];
        let channel_c = self.volume_lut[self.channel_output(&self.channel_c)];
        return (channel_a + channel_b + channel_c) / 3.0;
    }

    pub fn record_output(&mut self) {
        self.channel_a.record_sample((self.channel_output(&self.channel_a)) as i16);
        self.channel_b.record_sample((self.channel_output(&self.channel_b)) as i16);
        self.channel_c.record_sample((self.channel_output(&self.channel_c)) as i16);
    }

    pub fn execute_command(&mut self, command: u8, data: u8) {
        match command {
            0x0 => { 
                self.channel_a.tone.period_compare = (self.channel_a.tone.period_compare & 0xFF00) + data as u16;
            },
            0x1 => { 
                self.channel_a.tone.period_compare = (self.channel_a.tone.period_compare & 0x00FF) + ((data as u16 & 0xF) << 8);
            },
            0x2 => { 
                self.channel_b.tone.period_compare = (self.channel_b.tone.period_compare & 0xFF00) + data as u16;
            },
            0x3 => { 
                self.channel_b.tone.period_compare = (self.channel_b.tone.period_compare & 0x00FF) + ((data as u16 & 0xF) << 8);
            },
            0x4 => { 
                self.channel_c.tone.period_compare = (self.channel_c.tone.period_compare & 0xFF00) + data as u16;
            },
            0x5 => { 
                self.channel_c.tone.period_compare = (self.channel_c.tone.period_compare & 0x00FF) + ((data as u16 & 0xF) << 8);
            },
            0x6 => {
                self.noise.period_compare = data as u16 & 0x1F;
            },
            0x7 => {
                self.channel_a.tone_enabled =  (data & 0b0000_0001) == 0;
                self.channel_b.tone_enabled =  (data & 0b0000_0010) == 0;
                self.channel_c.tone_enabled =  (data & 0b0000_0100) == 0;
                self.channel_a.noise_enabled = (data & 0b0000_1000) == 0;
                self.channel_b.noise_enabled = (data & 0b0001_0000) == 0;
                self.channel_c.noise_enabled = (data & 0b0010_0000) == 0;
            },
            0x8 => {
                self.channel_a.envelope_enabled = (data & 0b0001_0000) != 0;
                self.channel_a.static_volume = data & 0xF;
            },
            0x9 => {
                self.channel_b.envelope_enabled = (data & 0b0001_0000) != 0;
                self.channel_b.static_volume = data & 0xF;
            },
            0xA => {
                self.channel_c.envelope_enabled = (data & 0b0001_0000) != 0;
                self.channel_c.static_volume = data & 0xF;
            },
            0xB => {
                self.envelope.period_compare = (self.envelope.period_compare & 0xFF00) + data as u16;
            },
            0xC => { 
                self.envelope.period_compare = (self.envelope.period_compare & 0x00FF) + ((data as u16 & 0xF) << 8);
            },
            0xD => {
                self.envelope.hold_flag =      (data & 0b0000_0001) != 0;
                self.envelope.alternate_flag = (data & 0b0000_0010) != 0;
                self.envelope.attack_flag =    (data & 0b0000_0100) != 0;
                self.envelope.continue_flag =  (data & 0b0000_1000) != 0;
                self.envelope.restart_envelope();


            },
            _ => {}
        }
    }
}