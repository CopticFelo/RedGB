use crate::{
    bus::Bus,
    cpu::{alu::*, operands::R8, reg_file::Flag},
    error::GBError,
};

pub fn add(opcode: u8, bus: &mut Bus) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(opcode == 0xC6 || opcode == 0xCE, opcode, 0, bus);
    let src = r8_param.read(bus)?;
    let mut opcode_name = String::new();
    let carry_flag = (read_bits(opcode, 3, 1) == 1 && bus.registers.read_flag(Flag::Carry)) as u8;
    let (res, carry) = if carry_flag == 1 {
        opcode_name += "adc ";
        let res1 = bus.registers.a.overflowing_add(src);
        let res2 = res1.0.overflowing_add(carry_flag);
        (res2.0, res1.1 || res2.1)
    } else {
        opcode_name += "add ";
        bus.registers.a.overflowing_add(src)
    };
    let half_carry = (bus.registers.a & 0xF) + (src & 0xF) + carry_flag > 0xF;
    let zero = res == 0;
    bus.registers
        .set_all_flags(&[zero as u8, 0, half_carry as u8, carry as u8])?;
    bus.registers.a = res;
    Ok(opcode_name + &r8_param.log())
}

pub fn sub(opcode: u8, bus: &mut Bus) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(opcode == 0xD6 || opcode == 0xDE, opcode, 0, bus);
    let src = r8_param.read(bus)?;
    let mut opcode_name = String::new();
    let carry_flag = (read_bits(opcode, 3, 1) == 1 && bus.registers.read_flag(Flag::Carry)) as u8;
    let (res, carry) = if carry_flag == 1 {
        opcode_name += "sbc ";
        let res1 = bus.registers.a.overflowing_sub(src);
        let res2 = res1.0.overflowing_sub(carry_flag);
        (res2.0, res1.1 || res2.1)
    } else {
        opcode_name += "sub ";
        bus.registers.a.overflowing_sub(src)
    };
    let half_carry = (bus.registers.a & 0xF)
        .wrapping_sub(src & 0xF)
        .wrapping_sub(carry_flag)
        > 0xF;
    let zero = res == 0;
    bus.registers
        .set_all_flags(&[zero as u8, 1, half_carry as u8, carry as u8])?;
    bus.registers.a = res;
    Ok(opcode_name + &r8_param.log())
}

pub fn and(opcode: u8, bus: &mut Bus) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(opcode == 0xE6, opcode, 0, bus);
    let src = r8_param.read(bus)?;
    bus.registers.a &= src;
    bus.registers
        .set_all_flags(&[(bus.registers.a == 0) as u8, 0, 1, 0])?;
    Ok(format!("and {}", r8_param.log()))
}

pub fn xor(opcode: u8, bus: &mut Bus) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(opcode == 0xEE, opcode, 0, bus);
    let src = r8_param.read(bus)?;
    bus.registers.a ^= src;
    bus.registers
        .set_all_flags(&[(bus.registers.a == 0) as u8, 0, 0, 0])?;
    Ok(format!("xor {}", r8_param.log()))
}

pub fn or(opcode: u8, bus: &mut Bus) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(opcode == 0xF6, opcode, 0, bus);
    let src = r8_param.read(bus)?;
    bus.registers.a |= src;
    bus.registers
        .set_all_flags(&[(bus.registers.a == 0) as u8, 0, 0, 0])?;
    Ok(format!("or {}", r8_param.log()))
}

pub fn cp(opcode: u8, bus: &mut Bus) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(opcode == 0xFE, opcode, 0, bus);
    let subtrahend = r8_param.read(bus)?;
    let half_carry = (bus.registers.a & 0xF) < (subtrahend & 0xF);
    let (res, carry) = bus.registers.a.overflowing_sub(subtrahend);
    let zero = res == 0;
    bus.registers
        .set_all_flags(&[zero as u8, 1, half_carry as u8, carry as u8])?;
    Ok(format!("cp {}", r8_param.log()))
}

/// inc r8 | inc hl | dec r8 | dec hl
pub fn inc_r8(opcode: u8, bus: &mut Bus, delta: i8) -> Result<String, GBError> {
    let r8_param = R8::get_r8_param(false, opcode, 3, bus);
    let value = r8_param.read(bus)?;
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
    r8_param.write(bus, res)?;
    bus.registers.set_all_flags(&[
        zero as u8,
        sub as u8,
        half_carry as u8,
        bus.registers.read_flag(Flag::Carry) as u8,
    ])?;
    Ok(opcode_name + &r8_param.log())
}

pub fn daa(bus: &mut Bus) -> Result<String, GBError> {
    let mut delta = 0;
    if bus.registers.read_flag(Flag::Subtract) {
        if bus.registers.read_flag(Flag::HalfCarry) {
            delta += 0x06;
        }
        if bus.registers.read_flag(Flag::Carry) {
            delta += 0x60;
        }
        bus.registers.a = bus.registers.a.wrapping_sub(delta);
        bus.registers.set_all_flags(&[
            (bus.registers.a == 0) as u8,
            bus.registers.read_flag(Flag::Subtract) as u8,
            0,
            bus.registers.read_flag(Flag::Carry) as u8,
        ])?;
    } else {
        // NOTE: don't know if the flag calculation are correct in this function
        if bus.registers.read_flag(Flag::HalfCarry) || (bus.registers.a & 0x0F) > 0x09 {
            delta += 0x06;
        }
        if bus.registers.read_flag(Flag::Carry) || bus.registers.a > 0x99 {
            delta += 0x60;
            bus.registers.set_flag(Flag::Carry, Some(true))?;
        }
        bus.registers.a = bus.registers.a.wrapping_add(delta);
        bus.registers.set_all_flags(&[
            (bus.registers.a == 0) as u8,
            bus.registers.read_flag(Flag::Subtract) as u8,
            0,
            bus.registers.read_flag(Flag::Carry) as u8,
        ])?;
    }
    Ok("daa".to_string())
}
