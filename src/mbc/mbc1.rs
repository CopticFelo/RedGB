use crate::{cpu::alu, rom::rom_info::ROMInfo};

use super::{Mbc, MbcFactory};
#[derive(Debug)]
pub struct MBC1 {
    rom_bank_count: u16,
    eram_enable: bool,
    bank_1: u8,
    bank_2: u8,
    mode: u8,
}
impl Mbc for MBC1 {
    fn write(
        &mut self,
        addr: u16,
        value: u8,
        rom_bank_a: &mut usize,
        rom_bank_b: &mut usize,
        eram_bank_index: &mut usize,
    ) {
        match addr {
            0x0..0x2000 => self.eram_enable = alu::read_bits(value, 0, 5) == 0xA,
            0x2000..0x4000 => {
                self.bank_1 = alu::read_bits(value, 0, 5);
                if [0, 0x20, 0x40, 0x60].contains(&value) {
                    self.bank_1 += 1
                }
            }
            0x4000..0x6000 => self.bank_2 = alu::read_bits(value, 0, 4),
            0x6000..0x8000 => {
                self.mode = alu::read_bits(value, 0, 1);
            }
            _ => (),
        }
        self.update_index(rom_bank_a, rom_bank_b, eram_bank_index);
    }
}
impl MbcFactory for MBC1 {
    fn new(rom_header: &ROMInfo) -> Self {
        Self {
            rom_bank_count: rom_header.rom_banks,
            eram_enable: false,
            bank_1: 1,
            bank_2: 0,
            mode: 1,
        }
    }
}

impl MBC1 {
    pub fn update_index(
        &self,
        rom_bank_a: &mut usize,
        rom_bank_b: &mut usize,
        eram_bank_index: &mut usize,
    ) {
        *rom_bank_b = (((self.bank_2 << 5) + self.bank_1) as u16 % self.rom_bank_count) as usize;
        if self.mode == 1 {
            *rom_bank_a = ((self.bank_2 << 5) as u16 % self.rom_bank_count) as usize;
            *eram_bank_index = self.bank_2 as usize;
        } else {
            *rom_bank_a = 0;
            *eram_bank_index = 0;
        }
    }
}
