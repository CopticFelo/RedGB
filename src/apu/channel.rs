use crate::cpu::alu;

const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 0, 0, 1],
    [1, 0, 0, 0, 0, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 0],
];

pub trait AudioChannel {
    fn tick(&mut self) -> f32;
    fn reset(&mut self, nrx2: u8, nrx3: u8, nrx4: u8);
}

#[derive(Default)]
pub struct PulseChannel {
    phase: u32,
    div: u32,
    period_timer: u8,
    pub(super) vol_timer: u8,
    pub length_timer: u8,
    pub(super) length_enable: bool,
    pub is_on: bool,
    pub(super) period_pace: u8,
    pub(super) period_step: u8,
    pub(super) period_inc: bool,
    pub(super) vol_period: u8,
    pub(super) vol_inc: bool,
    pub(super) period: u32,
    pub(super) duty_cycle: usize,
    pub(super) volume: u8,
}

impl AudioChannel for PulseChannel {
    fn tick(&mut self) -> f32 {
        if self.is_on {
            let sample = DUTY_TABLE[self.duty_cycle][self.phase as usize] as f32
                * (self.volume as f32 / 15.0);
            if self.div != 0 {
                self.div -= 1;
            } else {
                self.div = 2048 - self.period;
                self.phase = (self.phase + 1) & 7;
            }
            sample
        } else {
            0.0
        }
    }
    fn reset(&mut self, nrx2: u8, nrx3: u8, nrx4: u8) {
        self.is_on = true;
        if self.length_timer == 0 {
            self.length_timer = 64;
        }
        self.read_period(nrx3, nrx4);
        self.vol_period = alu::read_bits(nrx2, 0, 3);
        self.vol_timer = self.vol_period;
        self.vol_inc = alu::read_bits(nrx2, 3, 1) == 1;
        self.volume = alu::read_bits(nrx2, 4, 4);
        self.period_timer = if self.period_pace != 0 {
            self.period_pace
        } else {
            8
        };
    }
}

impl PulseChannel {
    pub fn read_period(&mut self, nrx3: u8, nrx4: u8) {
        let mut period = alu::read_bits(nrx4, 0, 3) as u16;
        period <<= 8;
        period |= alu::read_bits(nrx3, 0, 8) as u16;
        self.period = period as u32;
    }
    pub fn length_tick(&mut self) {
        if self.length_enable && self.length_timer != 0 {
            self.length_timer -= 1;
            if self.length_timer == 0 {
                self.is_on = false;
            }
        }
    }
    pub fn period_sweep(&mut self) {
        if !self.is_on {
            return;
        }
        self.period_timer -= 1;
        if self.period_timer == 0 {
            self.period_timer = if self.period_pace != 0 {
                self.period_pace
            } else {
                8
            };
            if self.period_pace != 0 {
                let step = self.period >> self.period_step;
                if self.period_inc {
                    self.period += step;
                } else {
                    self.period = self.period - (self.period / 2_u32.pow(self.period_step as u32));
                }
                if self.period > 2047 {
                    self.is_on = false;
                }
            }
        }
    }
    pub fn vol_sweep(&mut self) {
        if self.vol_period != 0 {
            self.vol_timer -= 1;
            if self.vol_timer == 0 {
                self.vol_timer = self.vol_period;
                if self.vol_inc && self.volume != 15 {
                    self.volume += 1;
                } else if !self.vol_inc && self.volume != 0 {
                    self.volume -= 1;
                }
            }
        }
    }
}
