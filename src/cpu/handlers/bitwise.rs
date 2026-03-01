use crate::{
    cpu::{alu, cpu_context::CpuContext, operands::R8, reg_file::Flag},
    error::GBError,
};

pub fn rotate_to_carry(opcode: u8, context: &mut CpuContext) -> Result<String, GBError> {
    let is_left = alu::read_bits(opcode, 3, 1) == 0;
    let through_carry = alu::read_bits(opcode, 4, 1) == 1;
    let r8_param = R8::get_r8_param(false, opcode, 0, context);
    let r8 = r8_param.read(context)?;
    let mut log = String::new();
    let (result, carry) = if is_left {
        log += &format!("{} ", if through_carry { "rl" } else { "rlc" });
        alu::rotate_left(
            r8,
            context
                .registers
                .read_flag(crate::cpu::reg_file::Flag::Carry),
            through_carry,
        )
    } else {
        log += &format!("{} ", if through_carry { "rr" } else { "rrc" });
        alu::rotate_right(
            r8,
            context
                .registers
                .read_flag(crate::cpu::reg_file::Flag::Carry),
            through_carry,
        )
    };
    log += &r8_param.log();
    r8_param.write(context, result)?;
    context
        .registers
        .set_all_flags(&[(result == 0) as u8, 0, 0, carry as u8])?;
    Ok(log)
}

pub fn shift_to_carry(opcode: u8, context: &mut CpuContext) -> Result<String, GBError> {
    let is_left = alu::read_bits(opcode, 3, 1) == 0;
    let is_logical = alu::read_bits(opcode, 4, 1) == 1;
    let r8_param = R8::get_r8_param(false, opcode, 0, context);
    let r8 = r8_param.read(context)?;
    let mut log = String::new();
    let (result, carry) = if is_logical {
        log += "srl ";
        let c = alu::read_bits(r8, 0, 1);
        let res = r8 >> 1;
        (res, c)
    } else if is_left {
        log += "sla ";
        let c = alu::read_bits(r8, 7, 1);
        let res = r8 << 1;
        (res, c)
    } else {
        log += "sra ";
        let c = alu::read_bits(r8, 0, 1);
        let last = alu::read_bits(r8, 7, 1);
        let res = (r8 >> 1) | (last << 7);
        (res, c)
    };
    log += &r8_param.log();
    r8_param.write(context, result)?;
    context
        .registers
        .set_all_flags(&[(result == 0) as u8, 0, 0, carry])?;
    Ok(log)
}

pub fn swap(opcode: u8, context: &mut CpuContext) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(false, opcode, 0, context);
    let r8 = r8_param.read(context)?;
    let high = alu::read_bits(r8, 4, 4);
    let result = (r8 << 4) | high;
    r8_param.write(context, result)?;
    context
        .registers
        .set_all_flags(&[(result == 0) as u8, 0, 0, 0])?;
    Ok(format!("swap {}", r8_param.log()))
}

pub fn test_bit(opcode: u8, context: &mut CpuContext) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(false, opcode, 0, context);
    let r8 = r8_param.read(context)?;
    let index = alu::read_bits(opcode, 3, 3);
    let result = alu::read_bits(r8, index, 1) == 0;
    context.registers.set_all_flags(&[
        result as u8,
        0,
        1,
        context.registers.read_flag(Flag::Carry) as u8,
    ])?;
    Ok(format!("bit {} {}", index, r8_param.log()))
}

pub fn reset_bit(opcode: u8, context: &mut CpuContext) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(false, opcode, 0, context);
    let r8 = r8_param.read(context)?;
    let index = alu::read_bits(opcode, 3, 3);
    let result = alu::set_bit(r8, index, false);
    r8_param.write(context, result)?;
    Ok(format!("res {} {}", index, r8_param.log()))
}

pub fn set_bit(opcode: u8, context: &mut CpuContext) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(false, opcode, 0, context);
    let r8 = r8_param.read(context)?;
    let index = alu::read_bits(opcode, 3, 3);
    let result = alu::set_bit(r8, index, true);
    r8_param.write(context, result)?;
    Ok(format!("set {} {}", index, r8_param.log()))
}
