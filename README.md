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
Be warned that most games still don't work, any game that depends on anything besides basic MBC1 functionality or Gameboy color games just straight up won't work

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
    - [x] MBC1 (Partial)
    - [ ] MBC2
    - [ ] MBC3
    - [ ] MBC4
    - [ ] MBC5
    - [ ] MBC6
    - [ ] MBC7
    - [ ] 3rd Party MBCs
- [ ] Sound
- [x] Input
- [ ] CGB Support
- [ ] GUI (Probably won't do)
