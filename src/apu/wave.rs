use crate::{apu::channel::AudioChannel, cpu::alu};

const VOLUME_TABLE: [f32; 4] = [0.0, 1.0, 0.5, 0.25];

#[derive(Default)]
pub struct WaveChannel {
    is_on: bool,
    pub dac_enable: bool,
    pub length_timer: u16,
    pub(super) length_enable: bool,
    phase: u8,
    div: u32,
    period: u32,
    wave_pattern: [u8; 32],
    volume: u8,
}

impl AudioChannel for WaveChannel {
    fn tick(&mut self) -> f32 {
        if self.volume > 3 {
            print!("");
        }
        let sample = (self.wave_pattern[self.phase as usize] as f32 / 15.0)
            * VOLUME_TABLE[self.volume as usize];
        if self.div != 0 {
            self.div -= 1;
        } else {
            self.phase = (self.phase + 1) & 31;
            self.div = 2048 - self.period;
        }
        sample
    }
    fn reset(&mut self, nrx2: u8, nrx3: u8, nrx4: u8) {
        self.is_on = true;
        if self.length_timer == 0 {
            self.length_timer = 256;
        }
        self.read_period(nrx3, nrx4);
        self.volume = alu::read_bits(nrx2, 5, 2);
        self.phase = 0;
    }
}

impl WaveChannel {
    pub fn length_tick(&mut self) {
        if self.length_enable && self.length_timer != 0 {
            self.length_timer -= 1;
            if self.length_timer == 0 {
                self.is_on = false;
            }
        }
    }
    pub fn read_period(&mut self, nrx3: u8, nrx4: u8) {
        let mut period = alu::read_bits(nrx4, 0, 3) as u16;
        period <<= 8;
        period |= alu::read_bits(nrx3, 0, 8) as u16;
        self.period = period as u32;
    }
    pub fn load_wave_pattern(&mut self, bytes: [u8; 16]) {
        for i in 0..32 {
            let bytes_index = i >> 1;
            self.wave_pattern[i] = if i & 1 == 0 {
                alu::read_bits(bytes[bytes_index], 4, 4)
            } else {
                alu::read_bits(bytes[bytes_index], 0, 4)
            }
        }
    }
}
