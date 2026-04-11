use crate::{
    bus::Bus,
    error::GBError,
    mbc::{Mbc, MbcFactory, mbc1::MBC1, mbc2::MBC2, mbc3::MBC3},
    rom::rom_info::ROMInfo,
};

#[derive(Debug)]
pub struct Memory {
    vram: Vec<Vec<u8>>,
    active_vram: usize,
    wram: Vec<Vec<u8>>,
    active_wram: usize,
    oam: Vec<u8>,
    pub io: Vec<u8>,
    hram: Vec<u8>,
    pub ie: u8,
    pub controller: Box<dyn Mbc>,
}

impl Memory {
    pub fn create_controller(rom: Vec<u8>, header_data: ROMInfo) -> Box<dyn Mbc> {
        match header_data.cartridge_type {
            0x0..=0x3 => Box::new(MBC1::new(rom, header_data)),
            0x5..=0x6 => Box::new(MBC2::new(rom, header_data)),
            0xF..0x19 => Box::new(MBC3::new(rom, header_data)),
            _ => todo!(),
        }
    }
    pub fn init_rom(rom: Vec<u8>, header_data: ROMInfo) -> Self {
        Self {
            vram: vec![vec![0; 0x2000]; 2],
            active_vram: 0,
            wram: vec![vec![0; 0x2000]; 8],
            active_wram: 1,
            oam: vec![0; 0xA0],
            io: vec![0; 0x80],
            hram: vec![0; 0x7F],
            ie: 0,
            controller: Self::create_controller(rom, header_data),
        }
    }
    pub fn dma_read(&self, addr: usize) -> Result<u8, GBError> {
        match addr {
            0x0000..=0x3FFF => Some(self.controller.read(addr)),
            0x4000..=0x7FFF => Some(self.controller.read(addr)),
            0x8000..=0x9FFF => self.vram[self.active_vram].get(addr - 0x8000).copied(),
            0xA000..=0xBFFF => Some(self.controller.read(addr)),
            0xC000..=0xCFFF => self.wram[0].get(addr - 0xC000).copied(),
            0xD000..=0xDFFF => self.wram[self.active_wram].get(addr - 0xD000).copied(),
            0xE000..=0xEFFF => self.wram[0].get(addr - 0xE000).copied(),
            0xF000..=0xFDFF => self.wram[self.active_wram].get(addr - 0xF000).copied(),
            0xFE00..=0xFE9F => self.oam.get(addr - 0xFE00).copied(),
            0xFEA0..=0xFEFF => Some(0),
            0xFF00..=0xFF7F => self.io.get(addr - 0xFF00).copied(),
            0xFF80..=0xFFFE => self.hram.get(addr - 0xFF80).copied(),
            0xFFFF => Some(self.ie),
            _ => None,
        }
        .ok_or(GBError::BadAddress(addr as u16))
    }
    pub fn dma_write(&mut self, addr: usize, value: u8) -> Result<(), GBError> {
        let opt_mem_ptr: Option<&mut u8> = match addr {
            0x0000..=0x1FFF => {
                self.controller.write(addr as u16, value);
                return Ok(());
            }
            0x2000..=0x3FFF => {
                self.controller.write(addr as u16, value);
                return Ok(());
            }
            0x4000..=0x7FFF => {
                self.controller.write(addr as u16, value);
                return Ok(());
            }
            0x8000..=0x9FFF => self.vram[self.active_vram].get_mut(addr - 0x8000),
            0xA000..=0xBFFF => {
                self.controller.write(addr as u16, value);
                return Ok(());
            }
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
    pub fn oam_transfer(bus: &mut Bus, addr: u8) {
        let src_addr = addr as usize * 0x100;
        let slice = match src_addr {
            0x0000..=0x3FFF => bus.memory.controller.read_range(src_addr, 0x9F),
            0x4000..=0x7FFF => bus.memory.controller.read_range(src_addr, 0x9F),
            0xC000..=0xCFFF => {
                let real_addr = src_addr - 0xC000;
                Some(&bus.memory.wram[0][real_addr..=real_addr + 0x9F])
            }
            0xD000..=0xDFFF => {
                let real_addr = src_addr - 0xD000;
                Some(&bus.memory.wram[0][real_addr..=real_addr + 0x9F])
            }
            0xE000..=0xEFFF => {
                let real_addr = src_addr - 0xE000;
                Some(&bus.memory.wram[0][real_addr..=real_addr + 0x9F])
            }
            _ => None,
        };
        if let Some(oam_data) = slice {
            bus.memory.oam.copy_from_slice(oam_data);
        }
        for _ in 0..160 {
            bus.tick();
        }
    }
}
