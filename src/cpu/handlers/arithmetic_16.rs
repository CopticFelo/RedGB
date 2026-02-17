use crate::{
    cpu::{
        alu,
        cpu_context::{self, CpuContext},
        operands::{R16, R16Type},
        reg_file::Flag,
    },
    error::GBError,
};

pub fn inc_r16(opcode: u8, context: &mut CpuContext, delta: i8) -> Result<(), GBError> {
    let r16_param = R16::new(opcode, 4, R16Type::R16)?;
    let r16 = r16_param.read(&context.registers);
    let result = r16 as i16 + delta as i16;
    r16_param.write(result as u16, &mut context.registers);
    context.clock.tick(&mut context.memory.io[0x44]);
    Ok(())
}

pub fn add_hl(opcode: u8, context: &mut CpuContext) -> Result<(), GBError> {
    let r16_param = R16::new(opcode, 4, R16Type::R16)?;
    let r16 = r16_param.read(&context.registers);
    let hl = alu::read_u16(&context.registers.l, &context.registers.h);
    let (result, carry) = hl.overflowing_add(r16);
    let half_carry = (hl & 0xFFF) + (r16 & 0xFFF) > 0xFFF;
    context.registers.set_all_flags(&[
        context.registers.read_flag(Flag::Zero) as u8,
        0,
        half_carry as u8,
        carry as u8,
    ])?;
    alu::write_u16(&mut context.registers.l, &mut context.registers.h, result);
    context.clock.tick(&mut context.memory.io[0x44]);
    Ok(())
}

pub fn add_sp_delta(context: &mut CpuContext) -> Result<(), GBError> {
    let delta = context.fetch() as i8;
    let result = context.registers.sp as i16 + delta as i16;
    context.registers.sp = result as u16;
    context.clock.tick(&mut context.memory.io[0x44]);
    let carry = (context.registers.sp & 0xFF) + (delta as u16 & 0xFF) > 0xFF;
    let half_carry = (context.registers.sp as u8 & 0xF) + (delta as u8 & 0xF) > 0xF;
    context
        .registers
        .set_all_flags(&[0, 0, half_carry as u8, carry as u8])?;
    context.clock.tick(&mut context.memory.io[0x44]);
    Ok(())
}
