pub mod mbc1;
pub mod mbc2;
use crate::{cpu::alu, rom::rom_info::ROMInfo};

pub trait Mbc: std::fmt::Debug {
    fn write(
        &mut self,
        addr: u16,
        value: u8,
        rom_bank_a: &mut usize,
        rom_bank_b: &mut usize,
        eram_bank_index: &mut usize,
    );
}
pub trait MbcFactory {
    fn new(header: &ROMInfo) -> Self
    where
        Self: Sized;
}
