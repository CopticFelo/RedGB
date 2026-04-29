#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use redgb::emulator;
use redgb::rom::{rom_info, rom_parser};
use rfd::FileDialog;
use std::path::PathBuf;
use std::{env, fs, io, io::Write};

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico")
            .set("InternalName", "REDGB.EXE")
            .set_version_info(winres::VersionInfo::PRODUCTVERSION, 0x0001000000000000);
        res.compile().unwrap();
    }
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    let mut rom_path: String = String::new();

    if args.len() < 2 {
        #[cfg(not(debug_assertions))]
        {
            print!("Select ROM File:");
            let rom_path_opt = FileDialog::new()
                .add_filter("Gameboy ROM", &["gb"])
                .set_directory(".")
                .pick_file();
            match rom_path_opt {
                Some(path) => {
                    rom_path = path.to_str().unwrap().to_string();
                }
                None => return,
            }
        }
        #[cfg(debug_assertions)]
        {
            rom_path = "/home/felo/dev/rust/RedGB/test_roms/rtc3test.gb".to_string();
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
