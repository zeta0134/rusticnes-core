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

    mirroring: Mirroring,
    vram: Vec<u8>,
}

const NSF_PLAYER: [Opcode; 1] = [
    Lda(Immediate(0x42))
];

impl NsfMapper {
    pub fn from_nsf(nsf: NsfFile) -> Result<NsfMapper, String> {
        let mut nsf_player = assemble(Vec::from(NSF_PLAYER))?;
        nsf_player.resize(0x1000, 0);

        return Ok(NsfMapper {
            prg: MemoryBlock::new(&nsf.prg, MemoryType::Ram),
            chr: vec![0u8; 0x2000],
            nsf_player: nsf_player,

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
            0xFFFC => {Some(0x00)}, // reset vector
            0xFFFD => {Some(0x50)},
            _ => None
        }
    }

    fn write_cpu(&mut self, address: u16, data: u8) {
        match address {
            /* complicated! */
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
