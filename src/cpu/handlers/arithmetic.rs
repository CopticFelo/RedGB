use crate::{
    cpu::{alu::*, cpu_context::CpuContext, operands::R8, reg_file::Flag},
    error::GBError,
};

pub fn add(opcode: u8, context: &mut CpuContext) -> Result<(), GBError> {
    let mut src = read_bits(opcode, 0, 3);
    let operand_str;
    if opcode == 0xC6 || opcode == 0xCE {
        src = context.fetch();
        operand_str = "imm8";
    } else if src == 6 {
        src = context.memory.read(
            &mut context.clock,
            read_u16(&context.registers.l, &context.registers.h),
        )?;
        operand_str = "[hl]";
    } else {
        src = *context.registers.match_r8(src)?;
        operand_str = "r8";
    }
    let addend = if read_bits(opcode, 3, 1) == 1 && context.registers.read_flag(Flag::Carry) {
        print!("adc ");
        src + 1
    } else {
        print!("add ");
        src
    };
    print!("{}", operand_str);
    let half_carry = (context.registers.a & 0xF) + (addend & 0xF) > 0xF;
    let (res, carry) = context.registers.a.overflowing_add(addend);
    let zero = res == 0;
    //FIX: Use set_all_flags()
    context
        .registers
        .set_flag(Flag::HalfCarry, Some(half_carry))?;
    context.registers.set_flag(Flag::Carry, Some(carry))?;
    context.registers.set_flag(Flag::Zero, Some(zero))?;
    context.registers.set_flag(Flag::Subtract, Some(false))?;
    context.registers.a = res;
    Ok(())
}

pub fn sub(opcode: u8, context: &mut CpuContext) -> Result<(), GBError> {
    // FIX: This Code block right here is repeated 3 times so far
    let mut src = read_bits(opcode, 0, 3);
    let operand_str;
    if opcode == 0xD6 || opcode == 0xDE {
        src = context.fetch();
        operand_str = "imm8";
    } else if src == 6 {
        src = context.memory.read(
            &mut context.clock,
            read_u16(&context.registers.l, &context.registers.h),
        )?;
        operand_str = "[hl]";
    } else {
        src = *context.registers.match_r8(src)?;
        operand_str = "r8";
    }
    let subtrahend = if read_bits(opcode, 3, 1) == 1 && context.registers.read_flag(Flag::Carry) {
        print!("sbc ");
        src + 1
    } else {
        print!("sub ");
        src
    };
    print!("{}", operand_str);
    let half_carry = (context.registers.a & 0xF) < (subtrahend & 0xF);
    let (res, carry) = context.registers.a.overflowing_sub(subtrahend);
    let zero = res == 0;
    //FIX: Use set_all_flags()
    context
        .registers
        .set_flag(Flag::HalfCarry, Some(half_carry))?;
    context.registers.set_flag(Flag::Carry, Some(carry))?;
    context.registers.set_flag(Flag::Zero, Some(zero))?;
    context.registers.set_flag(Flag::Subtract, Some(true))?;
    context.registers.a = res;
    Ok(())
}

//NOTE: Untested
pub fn and(opcode: u8, context: &mut CpuContext) -> Result<(), GBError> {
    print!("and ");
    let r8_param = R8::get_r8_param(opcode == 0xE6, opcode, 0, context);
    let src = r8_param.read(context)?;
    r8_param.log();
    context.registers.a &= src;
    context
        .registers
        .set_all_flags(&[(context.registers.a == 0) as u8, 0, 1, 0])?;
    Ok(())
}

//NOTE: Untested
pub fn xor(opcode: u8, context: &mut CpuContext) -> Result<(), GBError> {
    print!("xor ");
    let r8_param = R8::get_r8_param(opcode == 0xEE, opcode, 0, context);
    r8_param.log();
    let src = r8_param.read(context)?;
    context.registers.a ^= src;
    context
        .registers
        .set_all_flags(&[(context.registers.a == 0) as u8, 0, 0, 0])?;
    Ok(())
}

//NOTE: Untested
pub fn or(opcode: u8, context: &mut CpuContext) -> Result<(), GBError> {
    print!("or ");
    let r8_param = R8::get_r8_param(opcode == 0xF6, opcode, 0, context);
    let src = r8_param.read(context)?;
    r8_param.log();
    context.registers.a |= src;
    context
        .registers
        .set_all_flags(&[(context.registers.a == 0) as u8, 0, 0, 0])?;
    Ok(())
}

//NOTE: Untested
pub fn cp(opcode: u8, context: &mut CpuContext) -> Result<(), GBError> {
    //NOTE: This code is also valid for sub, probably need to do that as well there
    print!("cp ");
    let r8_param = R8::get_r8_param(opcode == 0xFE, opcode, 0, context);
    let subtrahend = r8_param.read(context)?;
    r8_param.log();
    let half_carry = (context.registers.a & 0xF) < (subtrahend & 0xF);
    let (res, carry) = context.registers.a.overflowing_sub(subtrahend);
    let zero = res == 0;
    context
        .registers
        .set_all_flags(&[zero as u8, 1, half_carry as u8, carry as u8])?;
    Ok(())
}

/// inc r8 | inc hl | dec r8 | dec hl
pub fn inc_r8(opcode: u8, context: &mut CpuContext, delta: i8) -> Result<(), GBError> {
    let r8_param = R8::get_r8_param(false, opcode, 3, context);
    let value = r8_param.read(context)?;
    let (half_carry, zero, sub, res): (bool, bool, bool, u8);
    if delta < 0 {
        print!("dec ");
        res = value.wrapping_sub(delta.unsigned_abs());
        half_carry = (value & 0xF) < (delta.unsigned_abs() & 0xF);
        sub = true
    } else {
        print!("inc ");
        res = value.wrapping_add(delta as u8);
        half_carry = (value & 0xF) + (delta as u8 & 0xF) > 0xF;
        sub = false
    }
    r8_param.log();
    zero = res == 0;
    r8_param.write(context, res)?;
    context.registers.set_all_flags(&[
        zero as u8,
        sub as u8,
        half_carry as u8,
        context.registers.read_flag(Flag::Carry) as u8,
    ])?;
    Ok(())
}

pub fn daa(context: &mut CpuContext) -> Result<(), GBError> {
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
    Ok(())
}
