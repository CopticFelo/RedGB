use log::trace;

use crate::cpu::{alu, cpu_context::CpuContext};

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
    pub fn tick(context: &mut CpuContext) {
        if context.t_cycles.abs_diff(context.gbtimer.div_last) >= 256 {
            context.memory.io[DIV] = context.memory.io[DIV].wrapping_add(1);
            context.gbtimer.div_last = context.t_cycles;
        }
        if alu::read_bits(context.memory.io[TAC], 2, 1) == 1 {
            Self::tima_step(context);
        }
    }
    pub fn tima_step(context: &mut CpuContext) {
        let inc_per_cycle = match alu::read_bits(context.memory.io[TAC], 0, 2) {
            0b00 => 1024,
            0b01 => 16,
            0b10 => 64,
            0b11 => 256,
            _ => unreachable!(),
        };
        if context.t_cycles.abs_diff(context.gbtimer.tima_last) >= inc_per_cycle {
            context.memory.io[TIMA] = {
                let (value, overflow) = context.memory.io[TIMA].overflowing_add(1);
                if overflow {
                    trace!("Timer Overflow");
                    context.memory.io[IF] = alu::set_bit(context.memory.io[IF], 2, true);
                    context.memory.io[TMA]
                } else {
                    value
                }
            };
            context.gbtimer.tima_last = context.t_cycles;
        }
    }
}
