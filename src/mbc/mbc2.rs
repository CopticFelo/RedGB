use std::any::Any;

use crate::{cpu::alu, error::GBError, mbc, rom::rom_info::ROMInfo};

use super::{Mbc, MbcFactory};
#[derive(Debug)]
pub struct MBC2 {
    rom_header: ROMInfo,
    rom_banks: Vec<Vec<u8>>,
    eram: Vec<Vec<u8>>,
    rom_bank_count: u16,
    eram_enable: bool,
    bank_1: u8,
    rom_index_b: usize,
}
impl Mbc for MBC2 {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
    fn load(&mut self) -> Result<(), GBError> {
        let data = match std::fs::read(mbc::save_path(&self.rom_header)) {
            Ok(data) => data,
            Err(_) => return Err(GBError::LoadError),
        };
        for (index, bank) in data.chunks(0x2000).enumerate() {
            if let Some(b) = self.eram.get_mut(index) {
                *b = bank.to_vec()
            }
        }
        Ok(())
    }
    fn save(&self) -> Result<(), GBError> {
        let mut save_data = vec![];
        for bank in &self.eram {
            for byte in bank {
                save_data.push(*byte);
            }
        }
        log::info!("Saving Game");
        match std::fs::write(mbc::save_path(&self.rom_header), save_data.as_slice()) {
            Ok(_) => Ok(()),
            Err(_) => Err(GBError::SaveError),
        }
    }
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
    fn read(&self, addr: usize) -> u8 {
        match addr {
            0x0..0x4000 => self.rom_banks[0].get(addr).copied().unwrap_or(0xFF),
            0x4000..0x8000 => self.rom_banks[self.rom_index_b]
                .get(addr - 0x4000)
                .copied()
                .unwrap_or(0xFF),
            0xA000..0xC000 => self.eram[0].get(addr - 0xA000).copied().unwrap_or(0xFF),
            _ => 0xFF,
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
                    let prev = self.eram_enable;
                    self.eram_enable = alu::read_bits(value, 0, 5) == 0xA;
                    if prev && !self.eram_enable && self.rom_header.cartridge_type == 6 {
                        self.save().expect("ERR");
                    }
                }
            }
        }
        self.update_index();
    }
}
impl MbcFactory for MBC2 {
    fn new(rom: Vec<u8>, rom_header: ROMInfo) -> Self {
        let mut rom_banks: Vec<Vec<u8>> = Vec::new();
        for bank in rom.chunks(0x4000) {
            rom_banks.push(bank.to_vec());
        }
        let mut mbc2 = Self {
            rom_banks,
            eram: vec![vec![0; 0x2000]; 1],
            rom_bank_count: rom_header.rom_banks,
            rom_header,
            eram_enable: false,
            bank_1: 1,
            rom_index_b: 1,
        };
        if mbc2.rom_header.cartridge_type == 6 {
            mbc2.load();
        }
        mbc2
    }
}

impl MBC2 {
    pub fn update_index(&mut self) {
        self.rom_index_b = (self.bank_1 % self.rom_bank_count as u8) as usize;
    }
}
