use std::{
    any::Any,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use sdl3::sys::breakpoint;

use crate::{
    cpu::alu,
    error::GBError,
    mbc::{self, Mbc, MbcFactory},
    rom::rom_info::ROMInfo,
};

/// Real time clock
#[derive(Debug)]
pub struct RTC {
    sub_seconds: u16,
    seconds: u8,
    minutes: u8,
    hours: u8,
    days: u16,
    carry: bool,
    pub latched_registers: [u8; 5],
    last_cycle: u64,
    is_halted: bool,
    is_latched: bool,
}

impl RTC {
    pub fn new() -> Self {
        Self {
            latched_registers: [0, 0, 0, 0, 0],
            last_cycle: 0,
            sub_seconds: 0,
            seconds: 0,
            minutes: 0,
            hours: 0,
            days: 0,
            carry: false,
            is_halted: true,
            is_latched: false,
        }
    }
    pub fn tick(&mut self, cycle_count: &u64) {
        if self.is_halted {
            self.last_cycle = *cycle_count;
            return;
        }
        if cycle_count.abs_diff(self.last_cycle) >= 128 {
            self.last_cycle = *cycle_count;
            self.sub_seconds += 1;
            if self.sub_seconds == 32768 {
                log::debug!(
                    "{}D {}H {}M {}S",
                    self.days,
                    self.hours,
                    self.minutes,
                    self.seconds
                );
                self.sub_seconds = 0;
                self.seconds += 1;
                if self.seconds == 60 {
                    self.seconds = 0;
                    self.minutes += 1;
                    if self.minutes == 60 {
                        self.minutes = 0;
                        self.hours += 1;
                        if self.hours == 24 {
                            self.hours = 0;
                            self.days += 1;
                            if self.days == 512 {
                                self.days = 0;
                                self.carry = true;
                            }
                        }
                    }
                }
            }
        }
    }
    pub fn start(&mut self) {
        self.is_halted = false;
    }
    pub fn halt(&mut self) {
        self.is_halted = true;
    }
    pub fn latch(&mut self) {
        self.is_latched = !self.is_latched;
        if self.is_latched {
            self.latched_registers = [
                self.seconds,
                self.minutes,
                self.hours,
                self.days as u8,
                (self.days >> 8) as u8 | ((self.carry as u8) << 7) | ((self.is_halted as u8) << 6),
            ];
        }
    }
    pub fn read(&self, idx: usize) -> u8 {
        let registers = if self.is_latched {
            &self.latched_registers
        } else {
            &[
                self.seconds,
                self.minutes,
                self.hours,
                self.days as u8,
                (self.days >> 8) as u8 | ((self.carry as u8) << 7) | ((self.is_halted as u8) << 6),
            ]
        };
        registers[idx - 8]
    }
    pub fn write(&mut self, idx: usize, value: u8) {
        match idx {
            0x8 => self.seconds = value,
            0x9 => self.minutes = value,
            0xA => self.hours = value,
            0xB => {
                self.days &= 255 << 8;
                self.days |= value as u16;
            }
            0xC => {
                self.days &= 255;
                self.days |= (value as u16) << 8;
                if self.is_halted != (alu::read_bits(value, 6, 1) == 1) {
                    self.is_halted = alu::read_bits(value, 6, 1) == 1;
                }
                if self.carry != (alu::read_bits(value, 7, 1) == 1) {
                    self.carry = alu::read_bits(value, 7, 1) == 1;
                }
            }
            _ => (),
        }
    }
    pub fn get_reg_slice(&self, latched: bool) -> [u8; 20] {
        let mut reg_vec: Vec<u8> = Vec::with_capacity(20);
        if latched {
            reg_vec.extend_from_slice(&(self.latched_registers[0] as u32).to_le_bytes());
            reg_vec.extend_from_slice(&(self.latched_registers[1] as u32).to_le_bytes());
            reg_vec.extend_from_slice(&(self.latched_registers[2] as u32).to_le_bytes());
            reg_vec.extend_from_slice(&(self.latched_registers[3] as u32).to_le_bytes());
            reg_vec.extend_from_slice(&(self.latched_registers[4] as u32).to_le_bytes());
        } else {
            reg_vec.extend_from_slice(&(self.seconds as u32).to_le_bytes());
            reg_vec.extend_from_slice(&(self.minutes as u32).to_le_bytes());
            reg_vec.extend_from_slice(&(self.hours as u32).to_le_bytes());
            reg_vec.extend_from_slice(&(self.days as u8 as u32).to_le_bytes());
            reg_vec.extend_from_slice(
                &(((self.days >> 8) as u8
                    | ((self.carry as u8) << 7)
                    | ((self.is_halted as u8) << 6)) as u32)
                    .to_le_bytes(),
            );
        }
        reg_vec.try_into().unwrap()
    }
    fn add_secs(&mut self, secs: u64) {
        let total_seconds = self.seconds as u64 + secs;
        self.seconds = (total_seconds % 60) as u8;

        let total_mins = self.minutes as u64 + (total_seconds / 60);
        self.minutes = (total_mins % 60) as u8;

        let total_hours = self.hours as u64 + (total_mins / 60);
        self.hours = (total_hours % 24) as u8;

        let total_days = self.days as u64 + (total_hours / 24);
        self.days = (total_days % 512) as u16;

        if total_days >= 512 {
            self.carry = true;
        }
    }
    pub fn load(&mut self, footer: &[u8; 48]) {
        self.seconds = u32::from_le_bytes(footer[0..4].try_into().unwrap()) as u8;
        self.minutes = u32::from_le_bytes(footer[4..8].try_into().unwrap()) as u8;
        self.hours = u32::from_le_bytes(footer[8..12].try_into().unwrap()) as u8;
        let day_low = u32::from_le_bytes(footer[12..16].try_into().unwrap()) as u8;
        let day_high = u32::from_le_bytes(footer[16..20].try_into().unwrap()) as u8;
        self.days = ((day_high & 1) as u16) << 8 | day_low as u16;
        self.carry = alu::read_bits(day_high, 7, 1) == 1;
        self.is_halted = alu::read_bits(day_high, 6, 1) == 1;
        self.latched_registers[0] = u32::from_le_bytes(footer[20..24].try_into().unwrap()) as u8;
        self.latched_registers[1] = u32::from_le_bytes(footer[24..28].try_into().unwrap()) as u8;
        self.latched_registers[2] = u32::from_le_bytes(footer[28..32].try_into().unwrap()) as u8;
        self.latched_registers[3] = u32::from_le_bytes(footer[32..36].try_into().unwrap()) as u8;
        self.latched_registers[4] = u32::from_le_bytes(footer[36..40].try_into().unwrap()) as u8;
        if footer[0..20] != footer[20..40] {
            self.is_latched = true;
        }
        if !self.is_halted {
            let timestamp = u64::from_le_bytes(footer[40..48].try_into().unwrap());
            let current_time = SystemTime::now();
            let current_timestamp = current_time.duration_since(UNIX_EPOCH).unwrap().as_secs();
            self.add_secs(current_timestamp.abs_diff(timestamp));
        }
    }
}

impl Default for RTC {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct MBC3 {
    rom: Vec<Vec<u8>>,
    eram: Vec<Vec<u8>>,
    rom_header: ROMInfo,
    selected_bank: u8,
    rtc_latch: bool,
    pub rtc: RTC,
    eram_rtc_select: u8,
    eram_enable: bool,
}

impl Mbc for MBC3 {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
    fn load(&mut self) -> Result<(), GBError> {
        let data = match std::fs::read(mbc::save_path(&self.rom_header)) {
            Ok(data) => data,
            Err(_) => return Err(GBError::LoadError),
        };
        for (index, bank) in data.chunks(0x2000).enumerate() {
            if let Some(b) = self.eram.get_mut(index)
                && bank.len() == 0x2000
            {
                *b = bank.to_vec()
            }
        }
        self.rtc
            .load(data[(data.len().saturating_sub(48))..].try_into().unwrap());
        Ok(())
    }
    fn save(&self) -> Result<(), GBError> {
        let mut save_data = vec![];
        for bank in &self.eram {
            for byte in bank {
                save_data.push(*byte);
            }
        }
        let current_time = SystemTime::now();
        let unix_time = current_time.duration_since(UNIX_EPOCH).unwrap().as_secs();
        save_data.extend_from_slice(&self.rtc.get_reg_slice(false));
        save_data.extend_from_slice(&self.rtc.get_reg_slice(true));
        save_data.extend_from_slice(&unix_time.to_le_bytes());
        log::info!("Saving Game");
        match std::fs::write(mbc::save_path(&self.rom_header), save_data.as_slice()) {
            Ok(_) => Ok(()),
            Err(_) => Err(GBError::SaveError),
        }
    }
    fn read_range(&self, addr: usize, len: usize) -> Option<&[u8]> {
        match addr {
            0x0..0x4000 => self.rom[0].get(addr..=(addr + len).min(0x4000)),
            0x4000..0x8000 => {
                let start = addr - 0x4000;
                let end = (start + len).min(0x8000);
                self.rom[self.selected_bank as usize].get(start..=end)
            }
            0xA000..0xC000 => {
                if self.eram_rtc_select <= 0x3 {
                    let start = addr - 0xA000;
                    let end = (start + len).min(0xC000);
                    self.eram[self.eram_rtc_select as usize].get(start..=end)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    fn read(&self, addr: usize) -> u8 {
        match addr {
            0x0..0x4000 => self.rom[0][addr],
            0x4000..0x8000 => self.rom[self.selected_bank as usize][addr - 0x4000],
            0xA000..0xC000 => {
                if self.eram_rtc_select <= 0x3 {
                    self.eram[self.eram_rtc_select as usize][addr - 0xA000]
                } else if self.eram_rtc_select >= 8 {
                    self.rtc.read(self.eram_rtc_select as usize)
                } else {
                    0xFF
                }
            }
            _ => 0xFF,
        }
    }
    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0..0x2000 => {
                let prev = self.eram_enable;
                self.eram_enable = alu::read_bits(value, 0, 5) == 0xA;
                if prev
                    && !self.eram_enable
                    && [0xF, 0x10, 0x13].contains(&self.rom_header.cartridge_type)
                {
                    self.save().expect("ERR");
                }
            }
            0x2000..0x4000 => {
                self.selected_bank = (value as u16 % self.rom_header.rom_banks) as u8;
                if self.selected_bank == 0 {
                    self.selected_bank = 1;
                }
                self.selected_bank &= 127;
                log::debug!("Bank {}/{}", self.selected_bank, self.rom_header.rom_banks);
            }
            0x4000..0x6000 => self.eram_rtc_select = value,
            0x6000..0x8000 => {
                if value == 1 && !self.rtc_latch {
                    self.rtc.latch();
                }
                self.rtc_latch = value == 1;
            }
            0xA000..0xC000 => {
                if self.eram_rtc_select <= 0x3 {
                    self.eram[self.eram_rtc_select as usize][addr as usize - 0xA000] = value;
                } else if self.eram_rtc_select >= 8 {
                    self.rtc.write(self.eram_rtc_select as usize, value);
                }
            }
            _ => (),
        }
    }
}
impl MbcFactory for MBC3 {
    fn new(rom: Vec<u8>, header: ROMInfo) -> Self
    where
        Self: Sized,
    {
        let mut rom_banks: Vec<Vec<u8>> = Vec::new();
        for bank in rom.chunks(0x4000) {
            rom_banks.push(bank.to_vec());
        }
        let mut mbc3 = Self {
            rom: rom_banks,
            eram: vec![vec![0; 0x2000]; header.mem_banks as usize + 2],
            rtc: RTC::default(),
            rom_header: header,
            selected_bank: 1,
            rtc_latch: false,
            eram_rtc_select: 0,
            eram_enable: false,
        };
        let _ = mbc3.load();
        mbc3
    }
}
