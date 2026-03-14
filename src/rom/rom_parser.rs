use crate::rom::rom_info::{CGBMode, ROMInfo};
use std::ops::Range;
use std::ops::RangeInclusive;

// https://gbdev.io/pandocs/The_Cartridge_Header.html#0104-0133--nintendo-logo
const NINTENDO: [u8; 48] = [
    0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D,
    0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
    0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E,
];

const NINTENDO_LOGO_RANGE: RangeInclusive<usize> = 0x104..=0x133;
const HEADER_SIZE: usize = 0x150;
const HEADER_RANGE: RangeInclusive<usize> = 0x134..=0x14C;
const TITLE_RANGE: Range<usize> = 0x134..0x143;
const CGB_FLAG_ADDR: usize = 0x143;
const SGB_FLAG_ADDR: usize = 0x146;
const CARTRIDGE_TYPE_ADDR: usize = 0x147;
const ROM_BANKS_ADDR: usize = 0x148;
const MEM_BANKS_ADDR: usize = 0x149;
const HEADER_CHECKSUM_ADDR: usize = 0x14D;
const ROM_CHECKSUM_RANGE: RangeInclusive<usize> = 0x14E..=0x14F;

/// Extracts important ROM data from ROM header and preforms validation
pub fn parse_rom_header(rom: &Vec<u8>) -> ROMInfo {
    assert!(rom.len() > HEADER_SIZE, "Invalid ROM File (File too short)");
    assert!(
        validate_rom(rom),
        "Invalid ROM File (No Nintendo Logo found)"
    );

    // https://gbdev.io/pandocs/The_Cartridge_Header.html#0134-0143--title
    let game_title = String::from_utf8_lossy(&rom[TITLE_RANGE]).to_string();

    // https://gbdev.io/pandocs/The_Cartridge_Header.html#0143--cgb-flag
    let cgb = rom[CGB_FLAG_ADDR];
    let cgb_mode = match cgb {
        0x80 => CGBMode::Color { exclusive: false },
        0xC0 => CGBMode::Color { exclusive: true },
        _ => CGBMode::Monochrome,
    };

    // https://gbdev.io/pandocs/The_Cartridge_Header.html#0146--sgb-flag
    let sgb = rom[SGB_FLAG_ADDR] == 0x3;

    // https://gbdev.io/pandocs/The_Cartridge_Header.html#0147--cartridge-type
    let cartridge_type = rom[CARTRIDGE_TYPE_ADDR];

    // https://gbdev.io/pandocs/The_Cartridge_Header.html#0148--rom-size
    let rom_banks = 2 * (1 << rom[ROM_BANKS_ADDR]);

    // https://gbdev.io/pandocs/The_Cartridge_Header.html#0149--ram-size
    let mem_banks = match rom[MEM_BANKS_ADDR] {
        0x0 => 0,
        0x1 => 1,
        0x3 => 4,
        0x4 => 16,
        0x5 => 8,
        _ => 0,
    };

    // https://gbdev.io/pandocs/The_Cartridge_Header.html#014d--header-checksum
    let header_checksum = rom[HEADER_CHECKSUM_ADDR];
    assert!(
        validate_header_checksum(&rom[HEADER_RANGE], header_checksum),
        "Invalid Header checksum"
    );

    // These two bytes form one 16-bit big endian number for the rom (global) checksum
    // https://gbdev.io/pandocs/The_Cartridge_Header.html#014e-014f--global-checksum
    let rom_checksum = {
        let bytes = &rom[ROM_CHECKSUM_RANGE];
        ((bytes[0] as u16) << 8) | bytes[1] as u16
    };

    let info = ROMInfo {
        title: game_title,
        cgb: cgb_mode,
        sgb,
        cartridge_type,
        rom_banks,
        mem_banks,
        header_checksum,
        rom_checksum,
    };
    println!("{}", info.title);
    println!("{}", info.cgb);
    info
}

fn validate_header_checksum(header: &[u8], checksum: u8) -> bool {
    let mut calculated_checksum: u8 = 0;
    for byte in header.iter() {
        calculated_checksum = calculated_checksum.wrapping_sub(*byte).wrapping_sub(1);
    }
    println!("Calculated checksum: {calculated_checksum:#x}");
    println!("Header checksum: {checksum:#x}");
    calculated_checksum == checksum
}

fn validate_rom(rom: &[u8]) -> bool {
    rom[NINTENDO_LOGO_RANGE] == NINTENDO
}
