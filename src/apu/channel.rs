use crate::cpu::alu;

const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 0],
];

pub trait AudioChannel {
    fn tick(&mut self, frame_sequencer: u8) -> f32;
}

#[derive(Default)]
pub struct PulseChannel {
    delta: u32,
    div: u32,
    vol_sweep: u8,
    period_sweep: u8,
    pub(super) length_timer: u8,
    pub(super) length_enable: bool,
    pub(super) is_on: bool,
    pub(super) period_pace: u8,
    pub(super) period_step: u8,
    pub(super) period_inc: bool,
    pub(super) vol_pace: u8,
    pub(super) vol_step: u8,
    pub(super) vol_inc: bool,
    pub(super) period: u32,
    pub(super) duty_cycle: usize,
    pub(super) volume: u8,
}

impl AudioChannel for PulseChannel {
    fn tick(&mut self, frame_sequencer: u8) -> f32 {
        if frame_sequencer & 1 == 0 && self.length_enable {
            if self.length_timer == 0 {
                self.is_on = false;
            } else {
                self.length_timer -= 0;
            }
        }
        if frame_sequencer == 2 || frame_sequencer == 6 {
            // self.period_sweep = (self.period_sweep + 1) & 7;
            // if self.period_sweep == self.period_pace {
            //     self.period_sweep();
            // }
            // self.vol_sweep = (self.vol_sweep + 1) & 7;
            // if self.vol_sweep == self.vol_pace {
            //     self.vol_sweep();
            // }
        }
        if self.is_on {
            let phase = self.delta & 7;
            let sample = if DUTY_TABLE[self.duty_cycle][phase as usize] == 0 {
                // -(self.volume as f32) / 15.0
                0.0
            } else {
                self.volume as f32 / 15.0
            };
            if self.div != 0 {
                self.div -= 1;
            } else if self.div == 0 {
                self.div = (2048 - self.period) * 4;
                self.delta += 1;
            }
            sample
        } else {
            0.0
        }
    }
}

impl PulseChannel {
    pub fn read_period(&mut self, nrx3: u8, nrx4: u8) {
        let mut period = alu::read_bits(nrx4, 0, 3) as u16;
        period <<= 8;
        period |= alu::read_bits(nrx3, 0, 8) as u16;
        self.period = period as u32;
    }
    fn period_sweep(&mut self) {
        let step = self.period >> self.period_step;
        if self.period_inc {
            self.period = self.period + step;
        } else {
            self.period = self.period - (self.period / 2_u32.pow(self.period_step as u32));
        }
        if self.period > 2047 {
            self.is_on = false;
        }
        // log::info!("period: {}", self.period);
    }
    fn vol_sweep(&mut self) {
        if self.vol_inc {
            self.volume = (self.volume + 1) & 15;
        } else {
            self.volume = self.volume.saturating_sub(1);
        }
        // log::info!("vol: {}", self.volume);
    }
}
