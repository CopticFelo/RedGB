use log::trace;

use crate::{bus::Bus, cpu::alu, mem::map::Memory};

const TIMA: usize = 0x05;
const TMA: usize = 0x06;
const DIV: usize = 0x04;
const TAC: usize = 0x07;
const IF: usize = 0x0F;

#[derive(Default)]
pub struct GBTimer {
    div_last: u64,
    tima_last: u64,
}

impl GBTimer {
    pub fn tick(&mut self, mem: &mut Memory, t_cycles: &u64) {
        if t_cycles.abs_diff(self.div_last) >= 256 {
            mem.io[DIV] = mem.io[DIV].wrapping_add(1);
            self.div_last = *t_cycles;
        }
        if alu::read_bits(mem.io[TAC], 2, 1) == 1 {
            self.tima_step(mem, t_cycles);
        }
    }
    pub fn tima_step(&mut self, mem: &mut Memory, t_cycles: &u64) {
        let inc_per_cycle = match alu::read_bits(mem.io[TAC], 0, 2) {
            0b00 => 1024,
            0b01 => 16,
            0b10 => 64,
            0b11 => 256,
            _ => unreachable!(),
        };
        if t_cycles.abs_diff(self.tima_last) >= inc_per_cycle {
            mem.io[TIMA] = {
                let (value, overflow) = mem.io[TIMA].overflowing_add(1);
                if overflow {
                    trace!("Timer Overflow");
                    mem.io[IF] = alu::set_bit(mem.io[IF], 2, true);
                    mem.io[TMA]
                } else {
                    value
                }
            };
            self.tima_last = *t_cycles;
        }
    }
}
