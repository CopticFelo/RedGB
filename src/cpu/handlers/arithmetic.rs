use crate::{
    cpu::{alu::*, cpu_context::CpuContext, operands::R8, reg_file::Flag},
    error::GBError,
};

pub fn add(opcode: u8, context: &mut CpuContext) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(opcode == 0xC6 || opcode == 0xCE, opcode, 0, context);
    let src = r8_param.read(context)?;
    let mut opcode_name = String::new();
    let addend = if read_bits(opcode, 3, 1) == 1 && context.registers.read_flag(Flag::Carry) {
        opcode_name += "adc ";
        src + 1
    } else {
        opcode_name += "add ";
        src
    };
    let half_carry = (context.registers.a & 0xF) + (addend & 0xF) > 0xF;
    let (res, carry) = context.registers.a.overflowing_add(addend);
    let zero = res == 0;
    context
        .registers
        .set_all_flags(&[zero as u8, 0, half_carry as u8, carry as u8])?;
    context.registers.a = res;
    Ok(opcode_name + &r8_param.log())
}

pub fn sub(opcode: u8, context: &mut CpuContext) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(opcode == 0xD6 || opcode == 0xDE, opcode, 0, context);
    let src = r8_param.read(context)?;
    let mut opcode_name = String::new();
    let subtrahend = if read_bits(opcode, 3, 1) == 1 && context.registers.read_flag(Flag::Carry) {
        opcode_name += "sbc ";
        src + 1
    } else {
        opcode_name += "sub ";
        src
    };
    let half_carry = (context.registers.a & 0xF) < (subtrahend & 0xF);
    let (res, carry) = context.registers.a.overflowing_sub(subtrahend);
    let zero = res == 0;
    context
        .registers
        .set_all_flags(&[zero as u8, 1, half_carry as u8, carry as u8])?;
    context.registers.a = res;
    Ok(opcode_name + &r8_param.log())
}

pub fn and(opcode: u8, context: &mut CpuContext) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(opcode == 0xE6, opcode, 0, context);
    let src = r8_param.read(context)?;
    context.registers.a &= src;
    context
        .registers
        .set_all_flags(&[(context.registers.a == 0) as u8, 0, 1, 0])?;
    Ok(format!("and {}", r8_param.log()))
}

pub fn xor(opcode: u8, context: &mut CpuContext) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(opcode == 0xEE, opcode, 0, context);
    let src = r8_param.read(context)?;
    context.registers.a ^= src;
    context
        .registers
        .set_all_flags(&[(context.registers.a == 0) as u8, 0, 0, 0])?;
    Ok(format!("xor {}", r8_param.log()))
}

pub fn or(opcode: u8, context: &mut CpuContext) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(opcode == 0xF6, opcode, 0, context);
    let src = r8_param.read(context)?;
    context.registers.a |= src;
    context
        .registers
        .set_all_flags(&[(context.registers.a == 0) as u8, 0, 0, 0])?;
    Ok(format!("or {}", r8_param.log()))
}

pub fn cp(opcode: u8, context: &mut CpuContext) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(opcode == 0xFE, opcode, 0, context);
    let subtrahend = r8_param.read(context)?;
    let half_carry = (context.registers.a & 0xF) < (subtrahend & 0xF);
    let (res, carry) = context.registers.a.overflowing_sub(subtrahend);
    let zero = res == 0;
    context
        .registers
        .set_all_flags(&[zero as u8, 1, half_carry as u8, carry as u8])?;
    Ok(format!("cp {}", r8_param.log()))
}

/// inc r8 | inc hl | dec r8 | dec hl
pub fn inc_r8(opcode: u8, context: &mut CpuContext, delta: i8) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(false, opcode, 3, context);
    let value = r8_param.read(context)?;
    let (half_carry, zero, sub, res): (bool, bool, bool, u8);
    let mut opcode_name = String::new();
    if delta < 0 {
        opcode_name += "dec ";
        res = value.wrapping_sub(delta.unsigned_abs());
        half_carry = (value & 0xF) < (delta.unsigned_abs() & 0xF);
        sub = true
    } else {
        opcode_name += "inc ";
        res = value.wrapping_add(delta as u8);
        half_carry = (value & 0xF) + (delta as u8 & 0xF) > 0xF;
        sub = false
    }
    zero = res == 0;
    r8_param.write(context, res)?;
    context.registers.set_all_flags(&[
        zero as u8,
        sub as u8,
        half_carry as u8,
        context.registers.read_flag(Flag::Carry) as u8,
    ])?;
    Ok(opcode_name + &r8_param.log())
}

pub fn daa(context: &mut CpuContext) -> Result<String, GBError> {
    let mut delta = 0;
    if context.registers.read_flag(Flag::Subtract) {
        if context.registers.read_flag(Flag::HalfCarry) {
            delta += 0x06;
        }
        if context.registers.read_flag(Flag::Carry) {
            delta += 0x60;
        }
        context.registers.a -= delta;
        context.registers.set_all_flags(&[
            (context.registers.a == 0) as u8,
            context.registers.read_flag(Flag::Subtract) as u8,
            0,
            context.registers.read_flag(Flag::Carry) as u8,
        ])?;
    } else {
        // NOTE: don't know if the flag calculation are correct in this function
        if context.registers.read_flag(Flag::HalfCarry) || (context.registers.a & 0x0F) > 0x09 {
            delta += 0x06;
            context.registers.set_flag(Flag::Carry, Some(true))?;
        }
        if context.registers.read_flag(Flag::Carry) || context.registers.a > 0x99 {
            delta += 0x60;
        }
        context.registers.a += delta;
        context.registers.set_all_flags(&[
            (context.registers.a == 0) as u8,
            context.registers.read_flag(Flag::Subtract) as u8,
            0,
            context.registers.read_flag(Flag::Carry) as u8,
        ])?;
    }
    Ok("daa".to_string())
}
