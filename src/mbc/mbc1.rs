use crate::{cpu::alu, rom::rom_info::ROMInfo};

use super::{Mbc, MbcFactory};
#[derive(Debug)]
pub struct MBC1 {
    rom_banks: Vec<Vec<u8>>,
    eram: Vec<Vec<u8>>,
    rom_bank_count: u16,
    eram_enable: bool,
    bank_1: u8,
    bank_2: u8,
    rom_index_a: usize,
    rom_index_b: usize,
    eram_index: usize,
    mode: u8,
}
impl Mbc for MBC1 {
    fn read_range(&self, addr: usize, len: usize) -> Option<&[u8]> {
        match addr {
            0x0..0x4000 => self.rom_banks[self.rom_index_a].get(addr..=((addr + len).min(0x4000))),
            0x4000..0x8000 => {
                let start = addr - 0x4000;
                let end = (start + len).min(0x8000);
                self.rom_banks[self.rom_index_b].get(start..=end)
            }
            0xA000..0xC000 => {
                let start = addr - 0xA000;
                let end = (start + len).min(0xC000);
                self.eram[self.eram_index].get(start..=end)
            }
            _ => None,
        }
    }
    fn read(&self, addr: usize) -> &u8 {
        match addr {
            0x0..0x4000 => self.rom_banks[self.rom_index_a].get(addr).unwrap_or(&0xFF),
            0x4000..0x8000 => self.rom_banks[self.rom_index_b]
                .get(addr - 0x4000)
                .unwrap_or(&0xFF),
            0xA000..0xC000 => self.eram[self.eram_index]
                .get(addr - 0xA000)
                .unwrap_or(&0xFF),
            _ => &0xFF,
        }
    }
    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0..0x2000 => self.eram_enable = alu::read_bits(value, 0, 5) == 0xA,
            0x2000..0x4000 => {
                self.bank_1 = alu::read_bits(value, 0, 5);
                if [0, 0x20, 0x40, 0x60].contains(&value) {
                    self.bank_1 += 1
                }
            }
            0x4000..0x6000 => self.bank_2 = alu::read_bits(value, 0, 2),
            0x6000..0x8000 => {
                self.mode = alu::read_bits(value, 0, 1);
            }
            0xA000..0xC000 => self.eram[self.eram_index][addr as usize - 0xA000] = value,
            _ => (),
        }
        self.update_index();
    }
}
impl MbcFactory for MBC1 {
    fn new(rom: Vec<u8>, rom_header: &ROMInfo) -> Self {
        let mut rom_banks: Vec<Vec<u8>> = Vec::new();
        for bank in rom.chunks(0x4000) {
            rom_banks.push(bank.to_vec());
        }
        Self {
            rom_banks,
            // HACK: This is still wrong
            eram: vec![vec![0; 0x2000]; rom_header.mem_banks as usize + 2],
            rom_bank_count: rom_header.rom_banks,
            eram_enable: false,
            bank_1: 1,
            bank_2: 0,
            mode: 1,
            rom_index_a: 0,
            rom_index_b: 1,
            eram_index: 0,
        }
    }
}

impl MBC1 {
    pub fn update_index(&mut self) {
        self.rom_index_b =
            (((self.bank_2 << 5) + self.bank_1) as u16 % self.rom_bank_count) as usize;
        if self.mode == 1 {
            self.rom_index_a = ((self.bank_2 << 5) as u16 % self.rom_bank_count) as usize;
            self.eram_index = self.bank_2 as usize;
        } else {
            self.rom_index_a = 0;
            self.eram_index = 0;
        }
    }
}
