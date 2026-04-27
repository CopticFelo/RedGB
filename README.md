# RedGB
### A GB emulator made in Rust
#### (I know this was made before but i am making my own just for fun/learning)

## Usage
```
git clone https://github.com/CopticFelo/RedGB.git
cd RedGB
cargo run -r -- path/to/rom.gb
```

## Game support
Most Gameboy (DMG) games work but some games have game-breaking glitches still (Prehistorik man, Pokemon Silver, Super mario land 2)
Gameboy color (CGB) games is still not supported at all

## Todo-list
- [x] CPU
- [x] Serial port
- [ ] PPU
    - [x] Background layer
    - [x] Window layer
    - [ ] Sprite layer
        - [x] Sprite rendering
        - [ ] Selection Priority
        - [x] Drawing Priority
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
