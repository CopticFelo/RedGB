# RedGB
### A GB emulator made in Rust
#### (I know this was made before but i made my own just for fun/learning)

## Special Thanks to Flavourtown (and Hackclub) for pushing me to complete this project, without them it wouldn't have even gotten past a basic SM83 emulator

## Showcase


https://github.com/user-attachments/assets/ba80226f-757b-419c-bceb-199a040d683e





## Usage
1. Install SDL3 from your package manager
- Ubuntu/Debian `sudo apt install libsdl3-dev`
- Fedora `sudo dnf install SDL3-devel SDL3`
- Arch `sudo pacman -S sdl3`
- MacOS `brew install sdl3`
2. Clone the repo and build it using cargo (don't forget the -r)
```
git clone https://github.com/CopticFelo/RedGB.git
cd RedGB
cargo run -r -- path/to/rom.gb
```
Or (if you can run them) you can try the release builds (you would probably still need SDL3 from your package manager)
### (Note: On MacOS after clicking on the executable .app it will show a "RedGB not opened" error, you need to go to System Settings > Privacy and Security, and scroll down till you find "RedGB was blocked to protect your mac" and click open anyways, this is because the app is not notarized and i don't have an apple dev account :<)

## Controls
| Keyboard | Original Gameboy |
| -------- | ---------------- | 
|    Z     |         A        |
|    X     |         B        |
|    C     |       Select     |
|  Return  |       Start      |
|Arrow keys|       D-Pad      |

## Game support
Most Gameboy (DMG) games work but some games have game-breaking glitches still (Prehistorik man, Pokemon Silver, Super mario land 2)
Gameboy color (CGB) games are not supported at all

### What works
Pretty much all monochrome gameboy hardware features except the audio noise channel and audio panning
meaning that with some hope most monochrome gameboy games work
Tested games include:
- The Legend of Zelda: Link's awakening
- Super Mario Land
- Tetris
- Mega Man V
- Mortal Kombat 1 & 2 (3 is broken)
- Killer instinct
- F-1 Race
- Dr. Mario
- Boxxle

### What doesn't
All gameboy color games don't work because this is strictly a DMG-only emulator
Some DMG games have game-breaking glitches (including but not limited to Prehistork man, Super Mario land 2, Mortal Kombat 3, Pokemon games)
Be also warned that the audio quality isn't that great
