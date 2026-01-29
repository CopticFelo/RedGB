use crate::cpu::clock::Clock;
use crate::cpu::cpu_context::CpuContext;
use crate::cpu::reg_file::{Modes, RegFile};
use crate::error::GBError;
use crate::mem::map;
use crate::rom::rom_info::ROMInfo;

pub fn init_emulation(rom: Vec<u8>, header_data: ROMInfo) -> Result<(), GBError> {
    let registers = RegFile::new(Modes::DMG);
    let memory = map::MemoryMap::init_rom(rom, header_data);
    let clock = Clock::default();
    let mut context = CpuContext::init(registers, memory, clock);
    context.start_exec_cycle()?;
    Ok(())
}
