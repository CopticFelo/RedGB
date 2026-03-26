use crate::{
    bus::Bus,
    cpu::{
        alu,
        operands::{R16, R16Type},
        reg_file::Flag,
    },
    error::GBError,
};

pub fn inc_r16(opcode: u8, bus: &mut Bus, delta: i8) -> Result<String, GBError> {
    let r16_param = R16::new(opcode, 4, R16Type::R16)?;
    let r16 = r16_param.read(&bus.registers);
    let result = (r16 as i16).wrapping_add(delta as i16);
    r16_param.write(result as u16, &mut bus.registers);
    bus.tick();
    Ok(format!(
        "{} {}",
        if delta < 0 { "dec" } else { "inc" },
        r16_param.log()
    ))
}

pub fn add_hl(opcode: u8, bus: &mut Bus) -> Result<String, GBError> {
    let r16_param = R16::new(opcode, 4, R16Type::R16)?;
    let r16 = r16_param.read(&bus.registers);
    let hl = alu::read_u16(&bus.registers.l, &bus.registers.h);
    let (result, carry) = hl.overflowing_add(r16);
    let half_carry = (hl & 0xFFF) + (r16 & 0xFFF) > 0xFFF;
    bus.registers.set_all_flags(&[
        bus.registers.read_flag(Flag::Zero) as u8,
        0,
        half_carry as u8,
        carry as u8,
    ])?;
    alu::write_u16(&mut bus.registers.l, &mut bus.registers.h, result);
    bus.tick();
    Ok("add hl".to_string())
}

pub fn add_sp_delta(bus: &mut Bus) -> Result<String, GBError> {
    let delta = bus.fetch() as i8;
    let result = (bus.registers.sp as i16).wrapping_add(delta as i16);
    let carry = (bus.registers.sp & 0xFF).wrapping_add(delta as u16 & 0xFF) > 0xFF;
    let half_carry = (bus.registers.sp as u8 & 0xF) + (delta as u8 & 0xF) > 0xF;
    bus.registers.sp = result as u16;
    bus.tick();
    bus.registers
        .set_all_flags(&[0, 0, half_carry as u8, carry as u8])?;
    bus.tick();
    Ok("add sp+e8".to_string())
}
