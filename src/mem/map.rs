use log::info;

use crate::{
    apu::channel::AudioChannel,
    bus::Bus,
    cpu::alu,
    error::GBError,
    mbc::{Mbc, MbcFactory, mbc1::MBC1},
    rom::rom_info::ROMInfo,
};

#[derive(Debug)]
pub struct Memory {
    rom_banks: Vec<Vec<u8>>,
    active_rom_a: usize,
    active_rom_b: usize,
    vram: Vec<Vec<u8>>,
    active_vram: usize,
    eram: Vec<Vec<u8>>,
    active_eram: usize,
    wram: Vec<Vec<u8>>,
    active_wram: usize,
    oam: Vec<u8>,
    pub io: Vec<u8>,
    hram: Vec<u8>,
    pub ie: u8,
    controller: Box<dyn Mbc>,
}

impl Memory {
    pub fn init_rom(rom: Vec<u8>, header_data: ROMInfo) -> Self {
        let mut rom_banks: Vec<Vec<u8>> = Vec::new();
        for bank in rom.chunks(0x4000) {
            rom_banks.push(bank.to_vec());
        }
        Self {
            active_rom_a: 0,
            active_rom_b: 1,
            rom_banks,
            vram: vec![vec![0; 0x2000]; 2],
            active_vram: 0,
            // HACK: This is wrong
            eram: vec![vec![0; 0x2000]; header_data.mem_banks as usize + 2],
            active_eram: 1,
            wram: vec![vec![0; 0x2000]; 8],
            active_wram: 1,
            oam: vec![0; 0xA0],
            io: vec![0; 0x80],
            hram: vec![0; 0x7F],
            ie: 0,
            controller: Box::new(MBC1::new(&header_data)),
        }
    }
    pub fn dma_read(&self, addr: usize) -> Result<u8, GBError> {
        match addr {
            0x0000..=0x3FFF => self.rom_banks[0].get(addr),
            0x4000..=0x7FFF => self.rom_banks[self.active_rom_b].get(addr - 0x4000),
            0x8000..=0x9FFF => self.vram[self.active_vram].get(addr - 0x8000),
            0xA000..=0xBFFF => self.eram[self.active_eram].get(addr - 0xA000),
            0xC000..=0xCFFF => self.wram[0].get(addr - 0xC000),
            0xD000..=0xDFFF => self.wram[self.active_wram].get(addr - 0xD000),
            0xE000..=0xEFFF => self.wram[0].get(addr - 0xE000),
            0xF000..=0xFDFF => self.wram[self.active_wram].get(addr - 0xF000),
            0xFE00..=0xFE9F => self.oam.get(addr - 0xFE00),
            0xFEA0..=0xFEFF => Some(&0),
            0xFF00..=0xFF7F => self.io.get(addr - 0xFF00),
            0xFF80..=0xFFFE => self.hram.get(addr - 0xFF80),
            0xFFFF => Some(&self.ie),
            _ => None,
        }
        .copied()
        .ok_or(GBError::BadAddress(addr as u16))
    }
    pub fn dma_write(&mut self, addr: usize, value: u8) -> Result<(), GBError> {
        let opt_mem_ptr: Option<&mut u8> = match addr {
            0x0000..=0x1FFF => {
                self.controller.write(
                    addr as u16,
                    value,
                    &mut self.active_rom_a,
                    &mut self.active_rom_b,
                    &mut self.active_eram,
                );
                return Ok(());
            }
            0x2000..=0x3FFF => {
                self.controller.write(
                    addr as u16,
                    value,
                    &mut self.active_rom_a,
                    &mut self.active_rom_b,
                    &mut self.active_eram,
                );
                return Ok(());
            }
            0x4000..=0x7FFF => {
                self.controller.write(
                    addr as u16,
                    value,
                    &mut self.active_rom_a,
                    &mut self.active_rom_b,
                    &mut self.active_eram,
                );
                return Ok(());
            }
            0x8000..=0x9FFF => self.vram[self.active_vram].get_mut(addr - 0x8000),
            0xA000..=0xBFFF => self.eram[self.active_eram].get_mut(addr - 0xA000),
            0xC000..=0xCFFF => self.wram[0].get_mut(addr - 0xC000),
            0xD000..=0xDFFF => self.wram[self.active_wram].get_mut(addr - 0xD000),
            0xE000..=0xEFFF => self.wram[0].get_mut(addr - 0xE000),
            0xF000..=0xFDFF => self.wram[self.active_wram].get_mut(addr - 0xF000),
            0xFE00..=0xFE9F => self.oam.get_mut(addr - 0xFE00),
            0xFEA0..=0xFEFF => {
                // https://gbdev.io/pandocs/Memory_Map.html#fea0feff-range
                // return Err(GBError::IllegalAddress(addr as u16));
                return Ok(());
            }
            0xFF00..=0xFF7F => self.io.get_mut(addr - 0xFF00),
            0xFF80..=0xFFFE => self.hram.get_mut(addr - 0xFF80),
            0xFFFF => Some(&mut self.ie),
            _ => None,
        };
        if let Some(mem_ptr) = opt_mem_ptr {
            *mem_ptr = value;
            Ok(())
        } else {
            Err(GBError::BadAddress(addr as u16))
        }
    }
    /// +160 M-C (640 T-C)
    pub fn oam_transfer(context: &mut Bus, addr: u8) {
        let src_addr = addr as usize * 0x100;
        let slice = match src_addr {
            0x0000..=0x3FFF => Some(&context.memory.rom_banks[0][src_addr..=src_addr + 0x9F]),
            0x4000..=0x7FFF => {
                let real_addr = src_addr - 0x4000;
                Some(
                    &context.memory.rom_banks[context.memory.active_rom_b]
                        [real_addr..=real_addr + 0x9F],
                )
            }
            0xC000..=0xCFFF => {
                let real_addr = src_addr - 0xC000;
                Some(&context.memory.wram[0][real_addr..=real_addr + 0x9F])
            }
            0xD000..=0xDFFF => {
                let real_addr = src_addr - 0xD000;
                Some(&context.memory.wram[0][real_addr..=real_addr + 0x9F])
            }
            0xE000..=0xEFFF => {
                let real_addr = src_addr - 0xE000;
                Some(&context.memory.wram[0][real_addr..=real_addr + 0x9F])
            }
            _ => None,
        };
        if let Some(oam_data) = slice {
            context.memory.oam.copy_from_slice(oam_data);
        }
        for _ in 0..160 {
            context.tick();
        }
    }
}
