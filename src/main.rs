use redgb::emulator;
use redgb::rom::{rom_info, rom_parser};
use std::io::Write;
use std::{env, fs, io};

fn main() {
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    let mut rom_path: String = String::new();
    if args.len() < 2 {
        #[cfg(not(debug_assertions))]
        {
            print!("Input ROM:");
            io::stdout().flush().unwrap();
            io::stdin()
                .read_line(&mut rom_path)
                .expect("Input error occoured");
            rom_path = rom_path.trim().to_string();
        }
        #[cfg(debug_assertions)]
        {
            rom_path = "/home/felo/dev/rust/RedGB/test_roms/tetris.gb".to_string();
        }
    } else {
        rom_path = args[1].clone();
    }
    println!("Reading input rom: {rom_path}");
    let rom = fs::read(rom_path).expect("Failed to read file");
    let info: rom_info::ROMInfo = rom_parser::parse_rom_header(&rom);
    match emulator::init_emulation(rom, info) {
        Ok(()) => (),
        Err(s) => eprintln!("{}", s),
    }
}
