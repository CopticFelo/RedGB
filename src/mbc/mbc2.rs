use crate::{cpu::alu, rom::rom_info::ROMInfo};

use super::{Mbc, MbcFactory};
#[derive(Debug)]
pub struct MBC2 {
    rom_bank_count: u16,
    eram_enable: bool,
    bank_1: u8,
}
impl Mbc for MBC2 {
    fn write(
        &mut self,
        addr: u16,
        value: u8,
        rom_bank_a: &mut usize,
        rom_bank_b: &mut usize,
        eram_bank_index: &mut usize,
    ) {
        if (addr >> 8) & 1 == 1 {
            self.bank_1 = alu::read_bits(value, 0, 4);
            if self.bank_1 == 0 {
                self.bank_1 += 1
            }
        } else {
            self.eram_enable = alu::read_bits(value, 0, 5) == 0xA;
        }
        self.update_index(rom_bank_a, rom_bank_b, eram_bank_index);
    }
}
impl MbcFactory for MBC2 {
    fn new(rom_header: &ROMInfo) -> Self {
        Self {
            rom_bank_count: rom_header.rom_banks,
            eram_enable: false,
            bank_1: 1,
        }
    }
}

impl MBC2 {
    pub fn update_index(
        &self,
        rom_bank_a: &mut usize,
        rom_bank_b: &mut usize,
        _eram_bank_index: &mut usize,
    ) {
        *rom_bank_a = 0;
        *rom_bank_b = (self.bank_1 % self.rom_bank_count as u8) as usize;
    }
}
