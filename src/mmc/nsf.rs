// The mapper used for NSF playback. This is planned to behave like a hardware
// player, so it will have some inherent limitations similar to most flashcarts.
// Reference capabilities: https://wiki.nesdev.com/w/index.php/NSF

use asm::*;
use asm::Opcode::*;
use asm::AddressingMode::*;
use memoryblock::MemoryBlock;
use memoryblock::MemoryType;
use mmc::mapper::*;
use mmc::mirroring;
use nsf::NsfFile;

pub struct NsfMapper {
    prg: MemoryBlock,
    chr: Vec<u8>,
    nsf_player: Vec<u8>,

    prg_rom_banks: Vec<usize>,

    // YOU WERE HERE. Next steps: implement banking, so the call to init_address reaches real code. Then,
    // see if that code executes and returns correctly; you should see a spinwait on page 0x5000, and might
    // also see some RAM values set.

    mirroring: Mirroring,
    vram: Vec<u8>,
}

const PPUCTRL: u16 = 0x2000;
const PPUMASK: u16 = 0x2001;
const PPUSTATUS: u16 = 0x2002;
const PPUSCROLL: u16 = 0x2005;
const PPUADDR: u16 = 0x2006;
const PPUDATA: u16 = 0x2007;

const APUSTATUS: u16 = 0x4015;
const APUFRAMECTRL: u16 = 0x4017;

const COLOR_BLACK: u8 = 0x0F;
const COLOR_WHITE: u8 = 0x30;

fn nsf_player(init_address: u16) -> Vec<Opcode> {
    vec![
        // Disable IRQ-based interrupts (We don't need them; NSF code by spec
        // shouldn't use them, and if it does, shenanigans.)
        Sei,

        Label(String::from("vwait1")),
        // Wait for NMI twice (PPU is not ready before this)
        Bit(Absolute(PPUSTATUS)),
        Bpl(RelativeLabel(String::from("vwait1"))),
        Label(String::from("vwait2")),
        Bit(Absolute(PPUSTATUS)),
        Bpl(RelativeLabel(String::from("vwait2"))),

        // We're in NMI now, so let's load in a better palette in slot 0
        Lda(Immediate(0x3F)),
        Sta(Absolute(PPUADDR)),
        Lda(Immediate(0x00)),
        Sta(Absolute(PPUADDR)),
        Lda(Immediate(COLOR_BLACK)),
        Sta(Absolute(PPUDATA)),
        Lda(Immediate(COLOR_WHITE)),
        Sta(Absolute(PPUDATA)),
        Sta(Absolute(PPUDATA)),
        Sta(Absolute(PPUDATA)),

        // Disable NMI, then set the scroll position and enable rendering
        Lda(Immediate(0b0000_1000)),
        Sta(Absolute(PPUCTRL)),
        Lda(Immediate(0x00)),
        Sta(Absolute(PPUSCROLL)),
        Sta(Absolute(PPUSCROLL)),
        Lda(Immediate(0b0000_1110)),
        Sta(Absolute(PPUMASK)),

        // Enable all channels)
        Lda(Immediate(0x0F)),
        Sta(Absolute(APUSTATUS)),
        // Set the frame counter to 4-step mode
        Lda(Immediate(0x40)),
        Sta(Absolute(APUFRAMECTRL)),
        // (bank initialization is handled by the mapper)
        // Load the first song index to A
        Lda(Immediate(0x00)),
        // Indicate NTSC mode in X
        Ldx(Immediate(0x00)),
        // Call the init subroutine
        Jsr(Absolute(init_address)),

        // For now, do nothing
        Label(String::from("wait_forever")),
        Lda(Immediate(0x00)), // TODO: use a jump here once that's implemented
        Beq(RelativeLabel(String::from("wait_forever"))),
    ]
} 

impl NsfMapper {
    pub fn from_nsf(nsf: NsfFile) -> Result<NsfMapper, String> {
        let mut nsf_player = assemble(Vec::from(nsf_player(nsf.header.init_address())))?;
        nsf_player.resize(0x1000, 0);

        let mut prg_rom = nsf.prg.clone();
        let mut prg_rom_banks = nsf.header.initial_banks();
        if !nsf.header.is_bank_switched() {
            if nsf.header.load_address() <= 0x8000 {
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

        return Ok(NsfMapper {
            prg: MemoryBlock::new(&prg_rom, MemoryType::Ram),
            chr: vec![0u8; 0x2000],
            nsf_player: nsf_player,

            prg_rom_banks: prg_rom_banks,

            mirroring: Mirroring::FourScreen,
            vram: vec![0u8; 0x1000],
        });
    }
}

impl Mapper for NsfMapper {
    fn mirroring(&self) -> Mirroring {
        return self.mirroring;
    }
    
    fn debug_read_cpu(&self, address: u16) -> Option<u8> {
        match address {
            0x5000 ..= 0x5FFF => Some(self.nsf_player[(address - 0x5000) as usize]),
            0x8000 ..= 0x8FFF => self.prg.banked_read(0x1000, self.prg_rom_banks[0], (address - 0x8000) as usize),
            0x9000 ..= 0x9FFF => self.prg.banked_read(0x1000, self.prg_rom_banks[1], (address - 0x9000) as usize),
            0xA000 ..= 0xAFFF => self.prg.banked_read(0x1000, self.prg_rom_banks[2], (address - 0xA000) as usize),
            0xB000 ..= 0xBFFF => self.prg.banked_read(0x1000, self.prg_rom_banks[3], (address - 0xB000) as usize),
            0xC000 ..= 0xCFFF => self.prg.banked_read(0x1000, self.prg_rom_banks[4], (address - 0xC000) as usize),
            0xD000 ..= 0xDFFF => self.prg.banked_read(0x1000, self.prg_rom_banks[5], (address - 0xD000) as usize),
            0xE000 ..= 0xEFFF => self.prg.banked_read(0x1000, self.prg_rom_banks[6], (address - 0xE000) as usize),
            0xF000 ..= 0xFFF9 => self.prg.banked_read(0x1000, self.prg_rom_banks[7], (address - 0xF000) as usize),
            0xFFFC => {Some(0x00)}, // reset vector
            0xFFFD => {Some(0x50)},
            _ => None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        match address {
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
