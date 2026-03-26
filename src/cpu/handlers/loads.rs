use crate::{
    bus::Bus,
    cpu::{alu, operands::R8},
    error::GBError,
};

pub fn load_r8(bus: &mut Bus, opcode: u8) -> Result<String, GBError> {
    let src_param = R8::get_r8_param(alu::read_bits(opcode, 6, 1) == 0, opcode, 0, bus);
    let src = src_param.read(bus)?;
    let dst_param = R8::get_r8_param(false, opcode, 3, bus);
    dst_param.write(bus, src)?;
    Ok(format!("ld {} {}", dst_param.log(), src_param.log()))
}
