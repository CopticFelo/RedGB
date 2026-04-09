use crate::{cpu::alu, rom::rom_info::ROMInfo};

use super::{Mbc, MbcFactory};
#[derive(Debug)]
pub struct MBC2 {
    rom_banks: Vec<Vec<u8>>,
    eram: Vec<Vec<u8>>,
    rom_bank_count: u16,
    eram_enable: bool,
    bank_1: u8,
    rom_index_b: usize,
}
impl Mbc for MBC2 {
    fn read_range(&self, addr: usize, len: usize) -> Option<&[u8]> {
        match addr {
            0x0..0x4000 => self.rom_banks[0].get(addr..=((addr + len).min(0x4000))),
            0x4000..0x8000 => {
                let start = addr - 0x4000;
                let end = (start + len).min(0x8000);
                self.rom_banks[self.rom_index_b].get(start..=end)
            }
            0xA000..0xC000 => {
                let start = addr - 0xA000;
                let end = (start + len).min(0xC000);
                self.eram[0].get(start..=end)
            }
            _ => None,
        }
    }
    fn read(&self, addr: usize) -> &u8 {
        match addr {
            0x0..0x4000 => self.rom_banks[0].get(addr).unwrap_or(&0xFF),
            0x4000..0x8000 => self.rom_banks[self.rom_index_b]
                .get(addr - 0x4000)
                .unwrap_or(&0xFF),
            0xA000..0xC000 => self.eram[0].get(addr - 0xA000).unwrap_or(&0xFF),
            _ => &0xFF,
        }
    }
    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xA000..0xA200 => self.eram[0][addr as usize - 0xA000] = alu::read_bits(value, 0, 4),
            0xA200..0xC000 => self.eram[0][addr as usize & 0x1FF] = alu::read_bits(value, 0, 4),
            _ => {
                if (addr >> 8) & 1 == 1 {
                    self.bank_1 = alu::read_bits(value, 0, 4);
                    if self.bank_1 == 0 {
                        self.bank_1 += 1
                    }
                } else {
                    self.eram_enable = alu::read_bits(value, 0, 5) == 0xA;
                }
            }
        }
        self.update_index();
    }
}
impl MbcFactory for MBC2 {
    fn new(rom: Vec<u8>, rom_header: &ROMInfo) -> Self {
        let mut rom_banks: Vec<Vec<u8>> = Vec::new();
        for bank in rom.chunks(0x4000) {
            rom_banks.push(bank.to_vec());
        }
        Self {
            rom_banks,
            eram: vec![vec![0; 0x2000]; 1],
            rom_bank_count: rom_header.rom_banks,
            eram_enable: false,
            bank_1: 1,
            rom_index_b: 1,
        }
    }
}

impl MBC2 {
    pub fn update_index(&mut self) {
        self.rom_index_b = (self.bank_1 % self.rom_bank_count as u8) as usize;
    }
}
