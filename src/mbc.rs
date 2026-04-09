pub mod mbc1;
pub mod mbc2;

use crate::rom::rom_info::ROMInfo;

pub trait Mbc: std::fmt::Debug {
    fn read_range(&self, addr: usize, len: usize) -> Option<&[u8]>;
    fn read(&self, addr: usize) -> &u8;
    fn write(&mut self, addr: u16, value: u8);
}
pub trait MbcFactory {
    fn new(rom: Vec<u8>, header: &ROMInfo) -> Self
    where
        Self: Sized;
}
