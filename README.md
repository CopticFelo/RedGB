# RedGB
### A GB emulator made in Rust
#### (I know this was made before but i am making my own just for fun/learning)

## Usage
```
git clone https://github.com/CopticFelo/RedGB.git
cd RedGB
cargo run -- path/to/rom.gb
```

## Game support
Most Gameboy (DMG) games work but games that rely on mid scanline effects or correct HBlank interrupt timing don't render correctly or straight up don't work (being worked on on the pixel-fifo branch)
Gameboy color (CGB) is still not supported at all

## Todo-list
- [x] CPU
- [x] Serial port
- [ ] PPU
    - [x] Background layer
    - [x] Window layer
    - [ ] Sprite layer
        - [x] Sprite rendering
        - [ ] Selection Priority
        - [ ] Drawing Priority
- [ ] Memory banking
    - [x] MBC1
    - [x] MBC2
    - [x] MBC3
    - [ ] MBC5
    - [ ] MBC6
    - [ ] MBC7
    - [ ] 3rd Party MBCs
- [ ] Sound
    - [x] Pulse channels (partially)
    - [x] Wave channel
    - [ ] Noise channel
- [x] Input
- [ ] CGB Support
- [ ] GUI (Probably won't do)
