use std::{any::Any, collections::VecDeque};

const NR52: usize = 0x26;
const NR10: usize = 0x10;
const NR11: usize = 0x11;
const NR12: usize = 0x12;
const NR13: usize = 0x13;
const NR14: usize = 0x14;
const NR21: usize = 0x16;
const NR22: usize = 0x17;
const NR23: usize = 0x18;
const NR24: usize = 0x19;
const T_CYCLES_PER_SAMPLE: f32 = 4194304.0 / 44100.0;

use crate::{
    apu::channel::{AudioChannel, PulseChannel},
    cpu::{alu, cpu_context::CpuContext},
};

pub struct APU {
    pub last_cycle: u64,
    pub accumulator: f32,
    pub frame_sequencer: u8,
    pub buffer: VecDeque<f32>,
    pub pulse_1: PulseChannel,
    pub pulse_2: PulseChannel,
}

impl APU {
    pub fn callback(&mut self, stream: &mut sdl3::audio::AudioStream, requested: i32) {
        let mut audio_slice = Vec::<f32>::with_capacity(requested as usize);
        for _ in 0..requested {
            let sample_opt = self.buffer.pop_front();
            match sample_opt {
                Some(sample) => audio_slice.push(sample),
                None => audio_slice.push(0.0),
            }
        }
        stream.put_data_f32(&audio_slice).unwrap();
    }
    pub fn init(context: &mut CpuContext) {
        // Trigger
        context.apu.pulse_1.is_on = alu::read_bits(context.memory.io[NR14], 7, 1) == 1;
        context.apu.pulse_2.is_on = alu::read_bits(context.memory.io[NR24], 7, 1) == 1;
        // Volume
        context.apu.pulse_1.volume = alu::read_bits(context.memory.io[NR12], 4, 4);
        context.apu.pulse_2.volume = alu::read_bits(context.memory.io[NR22], 4, 4);
        // Period
        context
            .apu
            .pulse_1
            .read_period(context.memory.io[NR13], context.memory.io[NR14]);
        context
            .apu
            .pulse_2
            .read_period(context.memory.io[NR23], context.memory.io[NR24]);
    }
    pub fn tick(context: &mut CpuContext) {
        context.apu.last_cycle += 4;
        context.apu.accumulator += 4.0;
        if context.apu.last_cycle == 8192 {
            context.apu.frame_sequencer = (context.apu.frame_sequencer + 1) & 7;
            context.apu.last_cycle = 0;
            if context.apu.frame_sequencer & 1 == 0 {
                context.apu.pulse_1.length_tick();
                context.apu.pulse_2.length_tick();
            }
            if context.apu.frame_sequencer == 7 {
                context.apu.pulse_1.vol_sweep();
                context.apu.pulse_2.vol_sweep();
            } else if context.apu.frame_sequencer == 2 || context.apu.frame_sequencer == 6 {
                // TODO: This function is still broken, fix it
                context.apu.pulse_1.period_sweep();
            }
        }
        // Channel 1
        context.apu.pulse_1.duty_cycle = match alu::read_bits(context.memory.io[0x11], 6, 2) {
            0b00 => 0,
            0b01 => 1,
            0b10 => 2,
            0b11 => 3,
            _ => unreachable!(),
        };
        context.apu.pulse_1.length_enable = alu::read_bits(context.memory.io[NR14], 6, 1) == 1;
        // Channel 2
        context.apu.pulse_2.duty_cycle = match alu::read_bits(context.memory.io[0x16], 6, 2) {
            0b00 => 0,
            0b01 => 1,
            0b10 => 2,
            0b11 => 3,
            _ => unreachable!(),
        };
        context.apu.pulse_2.length_enable = alu::read_bits(context.memory.io[NR24], 6, 1) == 1;
        let ch1 = context.apu.pulse_1.tick();
        let ch2 = context.apu.pulse_2.tick();
        while context.apu.accumulator >= T_CYCLES_PER_SAMPLE {
            context.apu.accumulator -= T_CYCLES_PER_SAMPLE;
            context.apu.buffer.push_back((ch1 + ch2) / 2.0);
        }
    }
}
