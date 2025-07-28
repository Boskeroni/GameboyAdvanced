# Gameboy Advanced Emulator

## Usage

Like my gameboy emulator, this uses justfile to make CL arguments easier. The commands are as follows (they do require folder's to be setup in specific ways)

### play

Requires a `roms/games/` folder to be present as this is where it looks for the file to run. **the .gba extension isn't necessary when passing the filename**

    just play [[ROM]]

### json-test

This is just used for testing, enables the `json-test` feature. Instead of running a file, it will run each test in [SingleStepTests' ARM7TDMI suite](https://github.com/SingleStepTests/ARM7TDMI).

    just json-test

### bios-test

This is just an alias for running games. **isn't used outside of testing**

    just bios-test

## Why I built this

I wanted to use this project to help improve my understanding of computer systems, as I felt the Gameboy emulator I have previously built seemed too distant from how I believed most systems worked. A GameboyAdvance Emulator seemed like a good next step.

I feel like I have taken a big interest in Nintendo consoles and will try my luck in creating a Gamecube emulator.

## Links to resources

- [GBATek](https://problemkaputt.de/gbatek.htm)
- [Jsmolka tests](https://github.com/jsmolka/gba-tests/tree/master)
- [Bios disassembly](https://github.com/Normmatt/gba_bios)
- [Cartride SRAM (GBATek was too brief for me)](https://densinh.github.io/DenSinH/emulation/2021/02/01/gba-eeprom.html)

## To-do list

- [x] have all json tests pass
- [x] all normal background modes working
- [x] DMA transfers
- [x] timers  
- [ ] implement Eeprom more  accurately
- [ ] allow CPU instructions to have custom timings
- [ ] implement affine backgrounds and sprites
- [ ] audio system

## Screenshots

[<video src="include/kirby.mp4" width="320" height="240" controls></video>]

pokemon red:
![pokemon red](https://github.com/Boskeroni/GameboyAdvanced/tree/master/include/pokemon-red.png)
