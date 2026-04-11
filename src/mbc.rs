pub mod mbc1;
pub mod mbc2;
pub mod mbc3;

use std::any::Any;
use std::fmt::Debug;

use crate::{error::GBError, rom::rom_info::ROMInfo};

pub trait Mbc: Debug + Any {
    fn as_any(&mut self) -> &mut dyn Any;
    fn read_range(&self, addr: usize, len: usize) -> Option<&[u8]>;
    fn save(&self) -> Result<(), GBError>;
    fn load(&mut self) -> Result<(), GBError>;
    fn read(&self, addr: usize) -> u8;
    fn write(&mut self, addr: u16, value: u8);
}
pub trait MbcFactory {
    fn new(rom: Vec<u8>, header: ROMInfo) -> Self
    where
        Self: Sized;
}
