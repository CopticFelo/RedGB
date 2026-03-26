use ringbuf::{SharedRb, storage::Heap, traits::Producer, wrap::caching::Caching};
use std::sync::Arc;

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
const NR30: usize = 0x1A;
const T_CYCLES_PER_SAMPLE: f32 = 4194304.0 / 44100.0;

use crate::{
    apu::{channel::AudioChannel, pulse::PulseChannel, wave::WaveChannel},
    cpu::alu,
    mem::map::MemoryMap,
};

pub struct APU {
    last_cycle: u64,
    accumulator: f32,
    frame_sequencer: u8,
    pub buffer: Caching<Arc<SharedRb<Heap<f32>>>, true, false>,
    pub pulse_1: PulseChannel,
    pub pulse_2: PulseChannel,
    pub wave: WaveChannel,
}

impl APU {
    pub fn new(buffer: Caching<Arc<SharedRb<Heap<f32>>>, true, false>) -> Self {
        APU {
            accumulator: 0.0,
            last_cycle: 0,
            frame_sequencer: 0,
            buffer,
            pulse_1: PulseChannel::default(),
            pulse_2: PulseChannel::default(),
            wave: WaveChannel::default(),
        }
    }
    pub fn init(&mut self, mem: &MemoryMap) {
        // Trigger
        self.pulse_1.is_on = alu::read_bits(mem.io[NR14], 7, 1) == 1;
        self.pulse_2.is_on = alu::read_bits(mem.io[NR24], 7, 1) == 1;
        self.wave.dac_enable = alu::read_bits(mem.io[NR30], 7, 1) == 1;
        // Volume
        self.pulse_1.volume = alu::read_bits(mem.io[NR12], 4, 4);
        self.pulse_2.volume = alu::read_bits(mem.io[NR22], 4, 4);
        // Period
        self.pulse_1.read_period(mem.io[NR13], mem.io[NR14]);
        self.pulse_2.read_period(mem.io[NR23], mem.io[NR24]);
    }
    pub fn tick(&mut self, mem: &MemoryMap) {
        self.last_cycle += 4;
        self.accumulator += 4.0;
        if self.last_cycle == 8192 {
            self.frame_sequencer = (self.frame_sequencer + 1) & 7;
            self.last_cycle = 0;
            if self.frame_sequencer & 1 == 0 {
                self.pulse_1.length_tick();
                self.pulse_2.length_tick();
                self.wave.length_tick();
            }
            if self.frame_sequencer == 7 {
                self.pulse_1.vol_sweep();
                self.pulse_2.vol_sweep();
            } else if self.frame_sequencer == 2 || self.frame_sequencer == 6 {
                // TODO: This function is still broken, fix it
                self.pulse_1.period_sweep();
            }
        }
        // Channel 1
        self.pulse_1.duty_cycle = match alu::read_bits(mem.io[0x11], 6, 2) {
            0b00 => 0,
            0b01 => 1,
            0b10 => 2,
            0b11 => 3,
            _ => unreachable!(),
        };
        self.pulse_1.length_enable = alu::read_bits(mem.io[NR14], 6, 1) == 1;
        // Channel 2
        self.pulse_2.duty_cycle = match alu::read_bits(mem.io[0x16], 6, 2) {
            0b00 => 0,
            0b01 => 1,
            0b10 => 2,
            0b11 => 3,
            _ => unreachable!(),
        };
        self.pulse_2.length_enable = alu::read_bits(mem.io[NR24], 6, 1) == 1;
        let ch1 = self.pulse_1.tick();
        let ch2 = self.pulse_2.tick();
        let mut ch3 = 0.0;
        if self.wave.dac_enable == true {
            ch3 = self.wave.tick();
            ch3 = (ch3 + self.wave.tick()) / 2.0;
        }
        while self.accumulator >= T_CYCLES_PER_SAMPLE {
            self.accumulator -= T_CYCLES_PER_SAMPLE;
            self.buffer.try_push((ch1 + ch2 + ch3) / 3.0);
        }
    }
}
