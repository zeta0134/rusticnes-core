// The mapper used for NSF playback. This is planned to behave like a hardware
// player, so it will have some inherent limitations similar to most flashcarts.
// Reference capabilities: https://wiki.nesdev.com/w/index.php/NSF

use apu::AudioChannelState;
use asm::*;
use asm::Opcode::*;
use asm::AddressingMode::*;
use memoryblock::MemoryBlock;
use memoryblock::MemoryType;
use mmc::mapper::*;
use mmc::mirroring;
use nsf::NsfFile;
use nsf::NsfHeader;

// various expansion audio chips
use mmc::vrc6::Vrc6PulseChannel;
use mmc::vrc6::Vrc6SawtoothChannel;

use apu::PulseChannelState;
use mmc::mmc5::Mmc5PcmChannel;

use mmc::fme7::YM2149F;

const PPUCTRL: u16 = 0x2000;
const PPUMASK: u16 = 0x2001;
const PPUSTATUS: u16 = 0x2002;
const PPUSCROLL: u16 = 0x2005;
const PPUADDR: u16 = 0x2006;
const PPUDATA: u16 = 0x2007;

const APUSTATUS: u16 = 0x4015;
const APUFRAMECTRL: u16 = 0x4017;

const COLOR_BLACK: u8 = 0x0F;
const COLOR_DARK_GREY: u8 = 0x2D;
const COLOR_LIGHT_GREY: u8 = 0x10;
const COLOR_WHITE: u8 = 0x30;

const PLAYER_COUNTER_COMPARE: u16 = 0x01FF;
const PLAYER_BUTTON_SCRATCH: u16 = 0x01FE;
const PLAYER_PLAYBACK_COUNTER: u16 = 0x4900;
const PLAYER_TRACK_SELECT: u16 = 0x4901;
const PLAYER_CURRENT_TRACK: u16 = 0x01FD;
const PLAYER_BUTTON_REPORT: u16 = 0x4902;
const PLAYER_RESET_BANKS: u16 = 0x4903;
const PLAYER_ORIGIN: u16 = 0x4A00;
const PLAYER_SIZE: u16 = 0x0200;
const PLAYER_END: u16 = PLAYER_ORIGIN + PLAYER_SIZE;

const JOYPAD1: u16 = 0x4016;

//const BUTTON_A: u8      = 1 << 7;
//const BUTTON_B: u8      = 1 << 6;
//const BUTTON_SELECT: u8 = 1 << 5;
//const BUTTON_START: u8  = 1 << 4;
//const BUTTON_UP: u8     = 1 << 3;
//const BUTTON_DOWN: u8   = 1 << 2;
const BUTTON_LEFT: u8   = 1 << 1;
const BUTTON_RIGHT: u8  = 1 << 0;

fn wait_for_ppu_ready() -> Opcode {
    return List(vec![
        Label(String::from("vwait1")),
        // Wait for NMI twice (PPU is not ready before this)
        Bit(Absolute(PPUSTATUS)),
        Bpl(RelativeLabel(String::from("vwait1"))),
        Label(String::from("vwait2")),
        Bit(Absolute(PPUSTATUS)),
        Bpl(RelativeLabel(String::from("vwait2"))),
    ]);
}

fn initialize_ppu() -> Opcode {
    return List(vec![
        // We're in NMI now, so let's load in a better palette in slot 0
        Lda(Immediate(0x3F)),
        Sta(Absolute(PPUADDR)),
        Lda(Immediate(0x00)),
        Sta(Absolute(PPUADDR)),
        Lda(Immediate(COLOR_BLACK)),
        Sta(Absolute(PPUDATA)),
        Lda(Immediate(COLOR_DARK_GREY)),
        Sta(Absolute(PPUDATA)),
        Lda(Immediate(COLOR_LIGHT_GREY)),
        Sta(Absolute(PPUDATA)),
        Lda(Immediate(COLOR_WHITE)),
        Sta(Absolute(PPUDATA)),

        // Disable NMI, then set the scroll position and enable rendering
        Lda(Immediate(0b0000_1000)),
        Sta(Absolute(PPUCTRL)),
        Lda(Immediate(0x00)),
        Sta(Absolute(PPUSCROLL)),
        Sta(Absolute(PPUSCROLL)),
        Lda(Immediate(0b0000_1110)),
        Sta(Absolute(PPUMASK)),
    ]);
}

fn initialize_apu() -> Opcode {
    return List(vec![
        // Enable all channels)
        Lda(Immediate(0x0F)),
        Sta(Absolute(APUSTATUS)),
        // Set the frame counter to 4-step mode
        Lda(Immediate(0x40)),
        Sta(Absolute(APUFRAMECTRL)),
    ]);
}

fn init_track(init_address: u16) -> Opcode {
    return List(vec![
        // (bank initialization is handled by the mapper)
        // Load the first song index to A
        Lda(Absolute(PLAYER_TRACK_SELECT)),
        Sta(Absolute(PLAYER_CURRENT_TRACK)),
        // Indicate NTSC mode in X
        Ldx(Immediate(0x00)),
        Jsr(Absolute(init_address)),
    ]);
}

fn poll_input() -> Opcode {
    return List(vec![
        // Repeatedly attempt the joypad read until we get the same value twice
        // works around a DPCM conflict
        Label(String::from("readjoy_safe")),
        Jsr(AbsoluteLabel(String::from("readjoy"))),
        Label(String::from("reread")),
        Lda(Absolute(PLAYER_BUTTON_SCRATCH)),
        Pha,
        Jsr(AbsoluteLabel(String::from("readjoy"))),
        Pla,
        Cmp(Absolute(PLAYER_BUTTON_SCRATCH)),
        Bne(RelativeLabel(String::from("reread"))),
        // Now the value in our scratch register is valid, so report it to
        // the mapper
        Sta(Absolute(PLAYER_BUTTON_REPORT)),
        Rts,

        // The joypad reading function; on its own this would be fine
        // if it weren't for DPCM
        Label(String::from("readjoy")),
        Lda(Immediate(0x01)),
        Sta(Absolute(JOYPAD1)),
        Sta(Absolute(PLAYER_BUTTON_SCRATCH)),
        Lsr(Accumulator),
        Sta(Absolute(JOYPAD1)),
        Label(String::from("joypadloop")),
        Lda(Absolute(JOYPAD1)),
        Lsr(Accumulator),
        Rol(Absolute(PLAYER_BUTTON_SCRATCH)),
        Bcc(RelativeLabel(String::from("joypadloop"))),
        Rts
    ]);
}

fn switch_tracks(init_address: u16) -> Opcode  {
    return List(vec![
        Label(String::from("switch_tracks")),
        Lda(Absolute(PLAYER_TRACK_SELECT)),
        Cmp(Absolute(PLAYER_CURRENT_TRACK)),
        Beq(RelativeLabel(String::from("done_switching_tracks"))),
        // save the current track which we are about to switch to
        Sta(Absolute(PLAYER_CURRENT_TRACK)),
        // Reset the banks prior to the init call
        // (The value written here is unimportant)
        Sta(Absolute(PLAYER_RESET_BANKS)),
        // load X for NTSC mode and call Init with the new track number
        Ldx(Immediate(0x00)),
        Jsr(Absolute(init_address)),
        Label(String::from("done_switching_tracks")),
        Rts
    ]);
}

fn playback_loop(play_address: u16) -> Opcode {
    return List(vec![
        // setup playback counter wait condition
        Lda(Absolute(PLAYER_PLAYBACK_COUNTER)),
        Sta(Absolute(PLAYER_COUNTER_COMPARE)),
        // push a 0x00 byte to the stack; this will become our preserved value of A
        Lda(Immediate(0x00)),
        Pha,
        Label(String::from("playback_loop")),
        // wait for the playback counter in the mapper to change to the next value
        Lda(Absolute(PLAYER_PLAYBACK_COUNTER)),
        Cmp(Absolute(PLAYER_COUNTER_COMPARE)),
        Beq(RelativeLabel(String::from("playback_loop"))),
        Sta(Absolute(PLAYER_COUNTER_COMPARE)),
        // Pop A off the stack, and call the play address
        Pla,
        Jsr(Absolute(play_address)), // not yet
        // Preserve A, since we are about to clobber it
        Pha,
        // Poll for input (clobbers only A)
        Jsr(AbsoluteLabel(String::from("readjoy_safe"))),
        Jsr(AbsoluteLabel(String::from("switch_tracks"))),
        // All done!
        Jmp(AbsoluteLabel(String::from("playback_loop"))),
    ]);
}

fn nsf_player(init_address: u16, play_address: u16) -> Vec<Opcode> {
    vec![
        // Disable IRQ-based interrupts (We don't need them; NSF code by spec
        // shouldn't use them, and if it does, shenanigans.)
        Sei,
        // Setup the stack frame at 0x01F0 (we'll use 0x01FF for our own single variable)
        Ldx(Immediate(0xF0)),
        Txs,

        wait_for_ppu_ready(),
        initialize_ppu(),
        initialize_apu(),
        init_track(init_address),

        // This loop will never exit, it drives the playback indefinitely
        playback_loop(play_address),

        // subroutines
        poll_input(),
        switch_tracks(init_address),
    ]
} 

pub struct NsfMapper {
    prg: MemoryBlock,
    chr: Vec<u8>,
    nsf_player: Vec<u8>,
    header: NsfHeader,

    current_track: u8,
    p1_held: u8,
    p1_pressed: u8,

    prg_rom_banks: Vec<usize>,
    playback_accumulator: f64,
    playback_period: f64,
    playback_counter: u8,

    mirroring: Mirroring,
    vram: Vec<u8>,

    vrc6_enabled: bool,
    vrc6_pulse1: Vrc6PulseChannel,
    vrc6_pulse2: Vrc6PulseChannel,
    vrc6_sawtooth: Vrc6SawtoothChannel,

    mmc5_enabled: bool,
    mmc5_multiplicand_a: u8,
    mmc5_multiplicand_b: u8,
    mmc5_pulse_1: PulseChannelState,
    mmc5_pulse_2: PulseChannelState,
    mmc5_audio_sequencer_counter: u16,
    mmc5_pcm_channel: Mmc5PcmChannel,
    mmc5_exram: Vec<u8>,

    s5b_enabled: bool,
    s5b_audio_command_select: u8,
    s5b_expansion_audio_chip: YM2149F,
}

impl NsfMapper {
    pub fn from_nsf(nsf: NsfFile) -> Result<NsfMapper, String> {
        let nsf_player_opcodes = nsf_player(nsf.header.init_address(), nsf.header.play_address());
        let mut nsf_player = assemble(nsf_player_opcodes, PLAYER_ORIGIN)?;
        nsf_player.resize(PLAYER_SIZE as usize, 0);

        let mut prg_rom = nsf.prg.clone();
        let mut prg_rom_banks = nsf.header.initial_banks();
        if !nsf.header.is_bank_switched() {
            if nsf.header.load_address() < 0x8000 {
                return Err(format!("Load address {} is below 0x8000, this conflicts with player implementation. Refusing to load.", nsf.header.load_address()));
            }

            // Coerce this ROM into a bank switched format anyway, so the mapper logic becomes simplified
            let mut padded_rom: Vec<u8> = Vec::new();
            padded_rom.resize((nsf.header.load_address() as usize) - 0x8000, 0);
            padded_rom.extend(prg_rom);
            padded_rom.resize(0x8000, 0);
            prg_rom = padded_rom;
            prg_rom_banks = vec![0, 1, 2, 3, 4, 5, 6, 7];
        }

        let ntsc_clockrate = 1786860.0;
        let cycles_per_play = (nsf.header.ntsc_playback_speed() as f64) * ntsc_clockrate / 1000000.0;
        let mut font_chr = include_bytes!("../../assets/troll8x8.chr").to_vec();
        font_chr.resize(0x2000, 0);

        return Ok(NsfMapper {
            prg: MemoryBlock::new(&prg_rom, MemoryType::Ram),
            chr: font_chr,
            nsf_player: nsf_player,
            header: nsf.header,
            playback_accumulator: 0.0,
            playback_period: cycles_per_play,
            playback_counter: 0,

            current_track: nsf.header.starting_song(),
            p1_held: 0,
            p1_pressed: 0,

            vrc6_enabled: nsf.header.vrc6(),
            vrc6_pulse1: Vrc6PulseChannel::new("Pulse 1"),
            vrc6_pulse2: Vrc6PulseChannel::new("Pulse 2"),
            vrc6_sawtooth: Vrc6SawtoothChannel::new(),

            mmc5_enabled: nsf.header.mmc5(),
            mmc5_multiplicand_a: 0,
            mmc5_multiplicand_b: 0,
            mmc5_pulse_1: PulseChannelState::new("Pulse 1", "MMC5", 1_789_773, false),
            mmc5_pulse_2: PulseChannelState::new("Pulse 2", "MMC5", 1_789_773, false),
            mmc5_audio_sequencer_counter: 0,
            mmc5_pcm_channel: Mmc5PcmChannel::new(),
            mmc5_exram: vec![0u8; 0x400],

            s5b_enabled: nsf.header.s5b(),
            s5b_audio_command_select: 0,
            s5b_expansion_audio_chip: YM2149F::new(),

            prg_rom_banks: prg_rom_banks,

            mirroring: Mirroring::FourScreen,
            vram: vec![0u8; 0x1000],
        });
    }

    pub fn draw_string(&mut self, x: usize, y: usize, width: usize, chars: Vec<u8>) {
        let mut dx = x;
        for c in chars {
            if dx < 32 && dx < (x + width) {
                if c >= 32 && c <= 127 {
                    let tile = y * 32 + dx;
                    let index = c - 32;
                    self.vram[tile] = index;
                }
            }
            dx += 1;
        }
    }

    pub fn draw_track_info(&mut self) {
        self.draw_string(21, 2, 9,  "RusticNES".as_bytes().to_vec());
        self.draw_string(20, 3, 10, "NSF Player".as_bytes().to_vec());

        self.draw_string(2, 5, 28, "- Title -".as_bytes().to_vec());
        let song_name = self.header.song_name();
        self.draw_string(2, 6, 28, song_name);

        self.draw_string(2, 8, 28, "- Artist -".as_bytes().to_vec());
        let artist_name = self.header.artist_name();
        self.draw_string(2, 9, 28, artist_name);

        self.draw_string(2, 11, 28, "- Copyright -".as_bytes().to_vec());
        let copyright_holder = self.header.copyright_holder();
        self.draw_string(2, 12, 28, copyright_holder);

        let track_display = format!("{}", self.current_track);
        self.draw_string(4, 16, 6, "Track:".as_bytes().to_vec());
        self.draw_string(11, 16, 3, "   ".as_bytes().to_vec());
        self.draw_string(11, 16, 3, track_display.as_bytes().to_vec());
    }

    pub fn process_input(&mut self) {
        if (self.p1_pressed & BUTTON_RIGHT) != 0 {
            if self.current_track < self.header.total_songs() {
                self.current_track += 1;
            }
        }
        if (self.p1_pressed & BUTTON_LEFT) != 0 {
            if self.current_track > 1 {
               self.current_track -= 1; 
            }
        }
    }

    pub fn update_gui(&mut self) {
        self.process_input();
        self.draw_track_info();
    }

    pub fn vrc6_output(&self) -> f64 {
        if !self.vrc6_enabled {
            return 0.0;
        }
        let pulse_1_output = if !self.vrc6_pulse1.debug_disable {self.vrc6_pulse1.output() as f64} else {0.0};
        let pulse_2_output = if !self.vrc6_pulse2.debug_disable {self.vrc6_pulse2.output() as f64} else {0.0};
        let sawtooth_output = if !self.vrc6_sawtooth.debug_disable {self.vrc6_sawtooth.output() as f64} else {0.0};
        let vrc6_combined_sample = (pulse_1_output + pulse_2_output + sawtooth_output) / 61.0;

        let nes_pulse_full_volume = 95.88 / ((8128.0 / 15.0) + 100.0);
        let vrc6_pulse_full_volume = 15.0 / 61.0;
        let vrc6_weight = nes_pulse_full_volume / vrc6_pulse_full_volume;
        return vrc6_combined_sample * vrc6_weight;
    }

    pub fn vrc6_write(&mut self, address: u16, data: u8) {
        match address {
            0x9000 => {self.vrc6_pulse1.write_register(0, data);},
            0x9001 => {self.vrc6_pulse1.write_register(1, data);},
            0x9002 => {self.vrc6_pulse1.write_register(2, data);},
            0x9003 => {
                self.vrc6_pulse1.write_register(3, data);
                self.vrc6_pulse2.write_register(3, data);
                self.vrc6_sawtooth.write_register(3, data);
            },
            0xA000 => {self.vrc6_pulse2.write_register(0, data);},
            0xA001 => {self.vrc6_pulse2.write_register(1, data);},
            0xA002 => {self.vrc6_pulse2.write_register(2, data);},
            // no 0xA003
            0xB000 => {self.vrc6_sawtooth.write_register(0, data);},
            0xB001 => {self.vrc6_sawtooth.write_register(1, data);},
            0xB002 => {self.vrc6_sawtooth.write_register(2, data);},
            _ => {}
        }
    }

    pub fn clock_vrc6(&mut self) {
        if self.vrc6_enabled {
            self.vrc6_pulse1.clock();
            self.vrc6_pulse2.clock();
            self.vrc6_sawtooth.clock();
        }
    }

    pub fn mmc5_write(&mut self, address: u16, data: u8) {
        if !self.mmc5_enabled {
            return;
        }
        let duty_table = [
            0b1000_0000,
            0b1100_0000,
            0b1111_0000,
            0b0011_1111,
        ];
        match address {
            0x5000 => {
                let duty_index =      (data & 0b1100_0000) >> 6;
                let length_disable =  (data & 0b0010_0000) != 0;
                let constant_volume = (data & 0b0001_0000) != 0;

                self.mmc5_pulse_1.duty = duty_table[duty_index as usize];
                self.mmc5_pulse_1.length_counter.halt_flag = length_disable;
                self.mmc5_pulse_1.envelope.looping = length_disable;
                self.mmc5_pulse_1.envelope.enabled = !(constant_volume);
                self.mmc5_pulse_1.envelope.volume_register = data & 0b0000_1111;
            },
            0x5002 => {
                let period_low = data as u16;
                self.mmc5_pulse_1.period_initial = (self.mmc5_pulse_1.period_initial & 0xFF00) | period_low;
            },
            0x5003 => {
                let period_high =  ((data & 0b0000_0111) as u16) << 8;
                let length_index = (data & 0b1111_1000) >> 3;

                self.mmc5_pulse_1.period_initial = (self.mmc5_pulse_1.period_initial & 0x00FF) | period_high;
                self.mmc5_pulse_1.length_counter.set_length(length_index);

                // Start this note
                self.mmc5_pulse_1.sequence_counter = 0;
                self.mmc5_pulse_1.envelope.start_flag = true;
            },
            0x5004 => {
                let duty_index =      (data & 0b1100_0000) >> 6;
                let length_disable =  (data & 0b0010_0000) != 0;
                let constant_volume = (data & 0b0001_0000) != 0;

                self.mmc5_pulse_2.duty = duty_table[duty_index as usize];
                self.mmc5_pulse_2.length_counter.halt_flag = length_disable;
                self.mmc5_pulse_2.envelope.looping = length_disable;
                self.mmc5_pulse_2.envelope.enabled = !(constant_volume);
                self.mmc5_pulse_2.envelope.volume_register = data & 0b0000_1111;
            },
            0x5006 => {
                let period_low = data as u16;
                self.mmc5_pulse_2.period_initial = (self.mmc5_pulse_2.period_initial & 0xFF00) | period_low;
            },
            0x5007 => {
                let period_high =  ((data & 0b0000_0111) as u16) << 8;
                let length_index =  (data & 0b1111_1000) >> 3;

                self.mmc5_pulse_2.period_initial = (self.mmc5_pulse_2.period_initial & 0x00FF) | period_high;
                self.mmc5_pulse_2.length_counter.set_length(length_index);

                // Start this note
                self.mmc5_pulse_2.sequence_counter = 0;
                self.mmc5_pulse_2.envelope.start_flag = true;
            },
            0x5010 => {
                self.mmc5_pcm_channel.read_mode =  (data & 0b0000_0001) != 0;
                self.mmc5_pcm_channel.irq_enable =  (data & 0b1000_0000) != 0;
            },
            0x5011 => {
                if !(self.mmc5_pcm_channel.read_mode) {
                    self.mmc5_pcm_channel.level = data;
                }
            },
            0x5015 => {
                self.mmc5_pulse_1.length_counter.channel_enabled  = (data & 0b0001) != 0;
                self.mmc5_pulse_2.length_counter.channel_enabled  = (data & 0b0010) != 0;
              
                if !(self.mmc5_pulse_1.length_counter.channel_enabled) {
                    self.mmc5_pulse_1.length_counter.length = 0;
                }
                if !(self.mmc5_pulse_2.length_counter.channel_enabled) {
                    self.mmc5_pulse_2.length_counter.length = 0;
                }
            }
            0x5205 => {self.mmc5_multiplicand_a = data;},
            0x5206 => {self.mmc5_multiplicand_b = data;},
            0x5C00 ..= 0x5FF5 => {
                self.mmc5_exram[(address - 0x5C00) as usize] = data;
            }
            _ => {}
        }
    }

    fn clock_mmc5(&mut self) {
        if !self.mmc5_enabled {
            return;
        }
        self.mmc5_audio_sequencer_counter += 1;
        if (self.mmc5_audio_sequencer_counter & 0b1) == 0 {
            self.mmc5_pulse_1.clock();
            self.mmc5_pulse_2.clock();
        }
        if self.mmc5_audio_sequencer_counter >= 7446 {
            self.mmc5_pulse_1.envelope.clock();
            self.mmc5_pulse_2.envelope.clock();
            self.mmc5_pulse_1.length_counter.clock();
            self.mmc5_pulse_2.length_counter.clock();
            // Note: MMC5 pulse channels don't support sweep. We're borrowing the implementation
            // from the underlying APU, but intentionally not clocking the sweep units.
            self.mmc5_audio_sequencer_counter = 0;
        }
    }

    fn snoop_mmc5(&mut self, address: u16) {
        if !self.mmc5_enabled {
            return;
        }
        // do the snoop PCM playback thing
        if self.mmc5_pcm_channel.read_mode {
            match address {
                0x8000 ..= 0xBFFF => {
                    self.mmc5_pcm_channel.level = self.debug_read_cpu(address).unwrap_or(0);
                },
                _ => {}
            }
        }
    }

    fn read_mmc5(&self, address: u16) -> Option<u8> {
        if !self.mmc5_enabled {
            return None;
        }

        // Handle MMC5 specific address spaces
        match address {
            0x5010 => {
                let mut pcm_status = 0;
                if self.mmc5_pcm_channel.read_mode {
                    pcm_status |= 0b0000_0001;
                }
                if self.mmc5_pcm_channel.irq_pending {
                    pcm_status |= 0b1000_0000;   
                }
                return Some(pcm_status)
            },
            0x5015 => {
                let mut pulse_status = 0;
                if self.mmc5_pulse_1.length_counter.length > 0 {
                    pulse_status += 0b0000_0001;
                }
                if self.mmc5_pulse_2.length_counter.length > 0 {
                    pulse_status += 0b0000_0010;
                }
                return Some(pulse_status);
            },
            0x5205 => {
                let result = self.mmc5_multiplicand_a as u16 * self.mmc5_multiplicand_b as u16;
                return Some((result & 0xFF) as u8);
            },
            0x5206 => {
                let result = self.mmc5_multiplicand_a as u16 * self.mmc5_multiplicand_b as u16;
                return Some(((result & 0xFF00) >> 8) as u8);
            },
            0x5C00 ..= 0x5FF5 => {
                return Some(self.mmc5_exram[(address - 0x5C00) as usize]);
            }
            _ => return None
        }
    }

    fn mmc5_output(&self) -> f64 {
        if !self.mmc5_enabled {
            return 0.0;
        }
        let pulse_1_output = (self.mmc5_pulse_1.output() as f64 / 15.0) - 0.5;
        let pulse_2_output = (self.mmc5_pulse_2.output() as f64 / 15.0) - 0.5;
        let mut pcm_output = (self.mmc5_pcm_channel.level as f64 / 256.0) - 0.5;
        if self.mmc5_pcm_channel.muted {
            pcm_output = 0.0;
        }

        return 
            (pulse_1_output + pulse_2_output) * 0.12 + 
            pcm_output * 0.25;
    }

    fn s5b_write(&mut self, address: u16, data: u8) {
        if !self.s5b_enabled {
            return;
        }
        match address {
            0xC000 => {
                self.s5b_audio_command_select = data & 0x0F;
            },
            0xE000 => {
                self.s5b_expansion_audio_chip.execute_command(self.s5b_audio_command_select, data);
            }
            _ => {}
        }
    }

    fn s5b_output(&self) -> f64 {
        if !self.s5b_enabled {
            return 0.0;
        }
        return (self.s5b_expansion_audio_chip.output() - 0.5) * -1.06;
    }

    fn clock_s5b(&mut self) {
        if !self.s5b_enabled {
            return;
        }
        self.s5b_expansion_audio_chip.clock();
    }
}

impl Mapper for NsfMapper {
    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }

    fn clock_cpu(&mut self) {
        self.playback_accumulator += 1.0;
        if self.playback_accumulator > self.playback_period {
            self.playback_counter = self.playback_counter.wrapping_add(1);
            self.playback_accumulator -= self.playback_period;
            self.update_gui();
        }

        self.clock_vrc6();
        self.clock_mmc5();
        self.clock_s5b();
    }

    fn mix_expansion_audio(&self, nes_sample: f64) -> f64 {
        return 
            self.vrc6_output() +
            self.mmc5_output() +
            self.s5b_output() +
            nes_sample;
    }

    fn channels(&self) ->  Vec<& dyn AudioChannelState> {
        let mut channels: Vec<& dyn AudioChannelState> = Vec::new();
        if self.vrc6_enabled {
            channels.push(&self.vrc6_pulse1);
            channels.push(&self.vrc6_pulse2);
            channels.push(&self.vrc6_sawtooth);
        }
        if self.mmc5_enabled {
            channels.push(&self.mmc5_pulse_1);
            channels.push(&self.mmc5_pulse_2);
            channels.push(&self.mmc5_pcm_channel);
        }
        if self.s5b_enabled {
            channels.push(&self.s5b_expansion_audio_chip.channel_a);
            channels.push(&self.s5b_expansion_audio_chip.channel_b);
            channels.push(&self.s5b_expansion_audio_chip.channel_c);
        }
        return channels;
    }

    fn channels_mut(&mut self) ->  Vec<&mut dyn AudioChannelState> {
        let mut channels: Vec<&mut dyn AudioChannelState> = Vec::new();
        if self.vrc6_enabled {
            channels.push(&mut self.vrc6_pulse1);
            channels.push(&mut self.vrc6_pulse2);
            channels.push(&mut self.vrc6_sawtooth);
        }
        if self.mmc5_enabled {
            channels.push(&mut self.mmc5_pulse_1);
            channels.push(&mut self.mmc5_pulse_2);
            channels.push(&mut self.mmc5_pcm_channel);
        }
        if self.s5b_enabled {
            channels.push(&mut self.s5b_expansion_audio_chip.channel_a);
            channels.push(&mut self.s5b_expansion_audio_chip.channel_b);
            channels.push(&mut self.s5b_expansion_audio_chip.channel_c);
        }
        return channels;
    }

    fn record_expansion_audio_output(&mut self) {
        if self.vrc6_enabled {
            self.vrc6_pulse1.record_current_output();
            self.vrc6_pulse2.record_current_output();
            self.vrc6_sawtooth.record_current_output();
        }
        if self.mmc5_enabled {
            self.mmc5_pulse_1.record_current_output();
            self.mmc5_pulse_2.record_current_output();
            self.mmc5_pcm_channel.record_current_output();
        }
        if self.s5b_enabled {
            self.s5b_expansion_audio_chip.record_output();
        }
    }
    
    fn read_cpu(&mut self, address: u16) -> Option<u8> {
        let data = self.debug_read_cpu(address);
        self.snoop_mmc5(address);
        return data;
    }

    fn debug_read_cpu(&self, address: u16) -> Option<u8> {
        match self.read_mmc5(address) {
            Some(data) => return Some(data),
            None => {}
        }

        match address {
            PLAYER_PLAYBACK_COUNTER => Some(self.playback_counter),
            PLAYER_TRACK_SELECT => Some(self.current_track - 1),
            PLAYER_ORIGIN ..= PLAYER_END => Some(self.nsf_player[(address - PLAYER_ORIGIN) as usize]),
            0x8000 ..= 0x8FFF => self.prg.banked_read(0x1000, self.prg_rom_banks[0], (address - 0x8000) as usize),
            0x9000 ..= 0x9FFF => self.prg.banked_read(0x1000, self.prg_rom_banks[1], (address - 0x9000) as usize),
            0xA000 ..= 0xAFFF => self.prg.banked_read(0x1000, self.prg_rom_banks[2], (address - 0xA000) as usize),
            0xB000 ..= 0xBFFF => self.prg.banked_read(0x1000, self.prg_rom_banks[3], (address - 0xB000) as usize),
            0xC000 ..= 0xCFFF => self.prg.banked_read(0x1000, self.prg_rom_banks[4], (address - 0xC000) as usize),
            0xD000 ..= 0xDFFF => self.prg.banked_read(0x1000, self.prg_rom_banks[5], (address - 0xD000) as usize),
            0xE000 ..= 0xEFFF => self.prg.banked_read(0x1000, self.prg_rom_banks[6], (address - 0xE000) as usize),
            0xF000 ..= 0xFFF9 => self.prg.banked_read(0x1000, self.prg_rom_banks[7], (address - 0xF000) as usize),
            0xFFFC => {Some(((PLAYER_ORIGIN & 0x00FF) >> 0) as u8)}, // reset vector
            0xFFFD => {Some(((PLAYER_ORIGIN & 0xFF00) >> 8) as u8)},
            _ => None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        match address {
            PLAYER_BUTTON_REPORT => {
                self.p1_pressed = data & (!self.p1_held);
                self.p1_held = data;
            },
            PLAYER_RESET_BANKS => {
                self.prg_rom_banks = self.header.initial_banks();
                if !self.header.is_bank_switched() {
                    self.prg_rom_banks = vec![0, 1, 2, 3, 4, 5, 6, 7];
                }
            },
            0x5FF8 => {self.prg_rom_banks[0] = data as usize},
            0x5FF9 => {self.prg_rom_banks[1] = data as usize},
            0x5FFA => {self.prg_rom_banks[2] = data as usize},
            0x5FFB => {self.prg_rom_banks[3] = data as usize},
            0x5FFC => {self.prg_rom_banks[4] = data as usize},
            0x5FFD => {self.prg_rom_banks[5] = data as usize},
            0x5FFE => {self.prg_rom_banks[6] = data as usize},
            0x5FFF => {self.prg_rom_banks[7] = data as usize},
            _ => {}
        }
        if self.vrc6_enabled {
            self.vrc6_write(address, data);
        }
        if self.mmc5_enabled {
            self.mmc5_write(address, data);
        }
        if self.s5b_enabled {
            self.s5b_write(address, data);
        }
    }

    fn debug_read_ppu(&self, address: u16) -> Option<u8> {
        match address {
            0x0000 ..= 0x1FFF => return Some(self.chr[address as usize]),
            0x2000 ..= 0x3FFF => return match self.mirroring {
                Mirroring::Horizontal => Some(self.vram[mirroring::horizontal_mirroring(address) as usize]),
                Mirroring::Vertical   => Some(self.vram[mirroring::vertical_mirroring(address) as usize]),
                Mirroring::FourScreen => Some(self.vram[mirroring::four_banks(address) as usize]),
                _ => None
            },
            _ => return None
        }
    }

    fn write_ppu(&mut self, address: u16, data: u8) {
        match address {
            0x0000 ..= 0x1FFF => {self.chr[address as usize] = data},
            0x2000 ..= 0x3FFF => match self.mirroring {
                Mirroring::Horizontal => self.vram[mirroring::horizontal_mirroring(address) as usize] = data,
                Mirroring::Vertical   => self.vram[mirroring::vertical_mirroring(address) as usize] = data,
                _ => {}
            },
            _ => {}
        }
    }
}
