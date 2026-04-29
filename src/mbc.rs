pub mod mbc1;
pub mod mbc2;
pub mod mbc3;

use std::fmt::Debug;
use std::{any::Any, path::PathBuf};

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

pub fn save_path(rom_header: &ROMInfo) -> PathBuf {
    let filename = format!("{}.sav", rom_header.title.trim_end_matches('\0'));

    let dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.copticfelo.redgb");

    std::fs::create_dir_all(&dir).ok();
    dir.join(filename)
}
