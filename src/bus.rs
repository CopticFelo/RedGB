use std::{sync::Arc, time::Instant};

use log::debug;
use ringbuf::{SharedRb, storage::Heap, wrap::caching::Caching};

use crate::{
    apu::{apu::APU, channel::AudioChannel},
    cpu::{alu, input::Joypad, reg_file::RegFile, timer::GBTimer},
    error::GBError,
    mbc::mbc3::MBC3,
    mem::map::Memory,
    ppu::ppu::PPU,
};

const SB: usize = 0x1;
const SC: usize = 0x2;

pub struct Bus {
    pub registers: RegFile,
    pub memory: Memory,
    pub t_cycles: u64,
    pub ppu: PPU,
    gbtimer: GBTimer,
    pub serial_message: Vec<u8>,
    pub joypad: Joypad,
    pub apu: APU,
}

impl Bus {
    pub fn init(
        registers: RegFile,
        memory: Memory,
        ppu: PPU,
        buffer: Caching<Arc<SharedRb<Heap<f32>>>, true, false>,
    ) -> Self {
        Self {
            registers,
            memory,
            t_cycles: 0,
            ppu,
            gbtimer: GBTimer::default(),
            serial_message: vec![],
            joypad: Joypad::default(),
            apu: APU::new(buffer),
        }
    }
    pub fn fetch(&mut self) -> u8 {
        let result = match self.read(self.registers.pc) {
            Ok(op) => op,
            // HACK: Probably improper error handling
            Err(s) => {
                debug!("{}", s);
                0x0
            }
        };
        self.registers.pc += 1;
        result
    }

    pub fn tick(&mut self) {
        self.t_cycles += 4_u64;
        self.ppu.tick(&mut self.memory, &self.t_cycles);
        self.gbtimer.tick(&mut self.memory, &self.t_cycles);
        self.apu.tick(&self.memory);

        if let Some(mbc3) = self.memory.controller.as_any().downcast_mut::<MBC3>() {
            mbc3.rtc.tick(&self.t_cycles);
        }

        if alu::read_bits(self.memory.io[SC], 7, 1) == 1 {
            self.memory.io[SB] <<= 1;
        }
    }
    pub fn read(&mut self, addr: u16) -> Result<u8, GBError> {
        self.tick();
        self.memory.dma_read(addr as usize)
    }
    pub fn write(&mut self, addr: u16, value: u8) -> Result<(), GBError> {
        self.tick();
        self.memory.dma_write(addr as usize, value)?;
        self.handle_io(addr as usize, value)?;
        Ok(())
    }
    fn handle_io(&mut self, addr: usize, value: u8) -> Result<(), GBError> {
        match addr {
            0xFF00 => {
                let byte = self.memory.io.get_mut(addr - 0xFF00);
                let reg = byte.unwrap();
                *reg = alu::set_bit(*reg, 4, alu::read_bits(value, 4, 1) == 1);
                *reg = alu::set_bit(*reg, 5, alu::read_bits(value, 5, 1) == 1);
                self.joypad.query_joypad(&mut self.memory);
                return Ok(());
            }
            0xFF01 => {
                self.serial_message.push(value);
            }
            0xFF04 => {
                let byte = self.memory.io.get_mut(addr - 0xFF00);
                *byte.unwrap() = 0;
                return Ok(());
            }
            0xFF11 => self.apu.pulse_1.length_timer = 64 - alu::read_bits(value, 0, 6),
            0xFF16 => self.apu.pulse_2.length_timer = 64 - alu::read_bits(value, 0, 6),
            0xFF14 => {
                if alu::read_bits(value, 7, 1) == 1 {
                    self.apu
                        .pulse_1
                        .reset(self.memory.io[0x12], self.memory.io[0x13], value);
                }
            }
            0xFF19 => {
                if alu::read_bits(value, 7, 1) == 1 {
                    self.apu
                        .pulse_2
                        .reset(self.memory.io[0x17], self.memory.io[0x18], value);
                }
            }
            0xFF1A => self.apu.wave.dac_enable = alu::read_bits(value, 7, 1) == 1,
            0xFF1B => self.apu.wave.length_timer = 256 - value as u16,
            0xFF1E => {
                if alu::read_bits(value, 7, 1) == 1 {
                    self.apu
                        .wave
                        .reset(self.memory.io[0x1C], self.memory.io[0x1D], value);
                }
            }
            0xFF30..0xFF40 => self
                .apu
                .wave
                .load_wave_pattern(self.memory.io[0x30..0x40].try_into().unwrap()),
            0xFF46 => {
                Memory::oam_transfer(self, value);
            }
            _ => (),
        };
        Ok(())
    }
}
