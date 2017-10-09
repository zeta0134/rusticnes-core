# RusticNES-Core

This is an NES emulator written in the Rust language. I began this project because I wanted to learn Rust, and having already written another emulator, I figured this was as good a way to introduce myself to the language as any.

The emulator is split up into the Core library (this repository) and platform specific shells which depend on this library. rusticnes-core contains the entire emulator with as few external dependencies as possible (presently just Rust's standard FileIO functions) so that it remains portable. All platform specific code is the responsibility of the shell.

If you're looking to compile and run a working copy of the emulator for PCs, you want RusticNES-SDL, which is the reference implementation. I've tested this on Windows and Arch Linux, and it should run on Mac, and any other platform that SDL supports. (If it does not please file a bug, I do not have a Mac to test with.) I may update this README with usage instructions for the core library after the project stabilizes a bit, but as it's in constant flux right now, I'll refer you to RusticNES-SDL for a reference implementation.

I'm striving for cycle accuracy with the emulator. While it works and runs some games, it presently falls far short of this goal. Here is the current state of the major systems:

## 6502 CPU

- All Official Instructions
- Some unofficial NOPs and STPs
- No unofficial instructions, the usage of these typically results in a crash due to undefined behavior.
- Cycle accurate, which should include additional reads / writes on dummy cycles. Testing is difficult due to unimplemented unofficial instructions, and shortcomings with the PPU.

## APU

- Feature complete as far as I can tell. Pulse, Triangle, Noise, and DMC are all working properly.
- DMC wait delay is not implemented
- Missing Frame timer even/odd jitter
- Audio is not mixed properly, relative channel volumes are therefore sometimes quite incorrect. It's close enough that things sound okay unless you know what to listen for.

## PPU

- Memory mapping, including cartridge mapper support, is all implemented and should be working.
- Nametable mirroring modes appear to work correctly, and are controlled by the mapper.
- Cycle timing is entirely wrong, entire system due for a rewrite.
- Register access mid-render is bugged, causing scrolling issues in many games, especially those which utilize vertical scrolling with a playfield split.
- Battletoads is... let's not talk about Battletoads.

