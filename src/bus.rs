use std::{sync::Arc, time::Instant};

use log::debug;
use ringbuf::{SharedRb, storage::Heap, wrap::caching::Caching};

use crate::{
    apu::apu::APU,
    cpu::{alu, input::Joypad, reg_file::RegFile, timer::GBTimer},
    mem::map::MemoryMap,
    ppu::ppu::PPU,
};

const SB: usize = 0x1;
const SC: usize = 0x2;

pub struct Bus {
    pub registers: RegFile,
    pub memory: MemoryMap,
    pub t_cycles: u64,
    pub ppu: PPU,
    pub gbtimer: GBTimer,
    pub serial_message: Vec<u8>,
    pub frame_drawn: bool,
    timer: Option<Instant>,
    pub joypad: Joypad,
    pub apu: APU,
}

impl Bus {
    pub fn init(
        registers: RegFile,
        memory: MemoryMap,
        ppu: PPU,
        buffer: Caching<Arc<SharedRb<Heap<f32>>>, true, false>,
    ) -> Self {
        Self {
            registers,
            memory,
            t_cycles: 0,
            ppu,
            gbtimer: GBTimer::default(),
            timer: None,
            serial_message: vec![],
            frame_drawn: false,
            joypad: Joypad::default(),
            apu: APU::new(buffer),
        }
    }
    pub fn fetch(&mut self) -> u8 {
        let result = match MemoryMap::read(self, self.registers.pc) {
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
        // trace!("cycles {}", self.t_cycles);
        PPU::tick(self);
        GBTimer::tick(self);
        self.apu.tick(&self.memory);

        if alu::read_bits(self.memory.io[SC], 7, 1) == 1 {
            self.memory.io[SB] <<= 1;
        }
    }
}
