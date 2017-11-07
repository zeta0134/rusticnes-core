# RusticNES-Core

This is an NES emulator written in the Rust language. I began this project because I wanted to learn Rust, and having already written another emulator, I figured this was as good a way to introduce myself to the language as any.

The emulator is split up into the Core library (this repository) and platform specific shells which depend on this library. rusticnes-core contains the entire emulator with as few external dependencies as possible (presently just Rust's standard FileIO functions) so that it remains portable. All platform specific code is the responsibility of the shell.

If you're looking to compile and run a working copy of the emulator for PCs, you want [RusticNES-SDL](https://github.com/zeta0134/rusticnes-sdl), which is the reference implementation. I've tested this on Windows and Arch Linux, and it should run on Mac, and any other platform that SDL supports. (If it does not please file a bug, I do not have a Mac to test with.) I may update this README with usage instructions for the core library after the project stabilizes a bit, but as it's in constant flux right now and lacks what I'd call a stable API, I'll instead refer you to [RusticNES-SDL](https://github.com/zeta0134/rusticnes-sdl) for a reference implementation.

I'm striving for cycle accuracy with the emulator. While it works and runs many games, it presently falls short of this goal. I am presently most focused on getting the base emulator to run properly, and pass various [accuracy tests](http://tasvideos.org/EmulatorResources/NESAccuracyTests.html). Mapper support should be easy to add as I go. Here is the current state of the major systems:

## 6502 CPU

- All Official Instructions.
- Some unofficial NOPs and all STPs.
- No unofficial instructions, the usage of these typically results in a crash due to undefined behavior.
- Mostly cycle accurate, which should include additional reads / writes on dummy cycles.
- Missing proper read delay implementation, needed for DMC DMA read delay, and proper interaction between DMC and OAM DMA during simultaneous operation.

## APU

- Feature complete as far as I can tell. Pulse, Triangle, Noise, and DMC are all working properly.
- DMC wait delay is not implemented.
- Audio is not mixed properly, relative channel volumes are therefore sometimes quite incorrect. It's close enough that things sound okay unless you know what to listen for.
- No interpolation or filtering, which can make especially high frequencies sound a bit off. The APU is producing the correct output, but the subsequent clamping to 44.1 KHz introduces artifacts.

## PPU

- Memory mapping, including cartridge mapper support, is all implemented and should be working.
- Nametable mirroring modes appear to work correctly, and are controlled by the mapper.
- Cycle timing should be very close to accurate. Tricky games like Battletoads appear to run correctly, though there may still be bugs here and there.
- Sprite overflow is implemented correctly. The sprite overflow bug is not, so games relying on the behavior of the sprite overflow flag will encounter accuracy problems relative to real hardware.

## Input

- A single Standard Controller plugged into port 1 is implemented. 
- Multiple controllers and additional peripheral support (Light Zapper, Track and Field Mat, Knitting Machine, etc) is planned, but not implemented.

## Mappers

- Currently supported: AxROM, GxROM, MMC1, MMC3, NROM, UxROM
- Currently unsupported: Everything else.
- MMC1 does not ignore writes on successive cycles, causing issues with some games.
- FDS and non-NTSC NES features (PAL, Vs System, etc) are entirely unsupported.
