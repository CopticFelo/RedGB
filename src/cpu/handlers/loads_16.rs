use crate::{
    bus::Bus,
    cpu::{
        alu,
        operands::{R16, R16Type},
    },
    error::GBError,
};

pub fn load_r16_imm16(bus: &mut Bus, opcode: u8) -> Result<String, GBError> {
    let param = R16::new(opcode, 4, R16Type::R16)?;
    let value = alu::read_u16(&bus.fetch(), &bus.fetch());
    param.write(value, &mut bus.registers);
    Ok(format!("ld {} {:#X}", param.log(), value))
}

pub fn load_r16mem_a(opcode: u8, bus: &mut Bus) -> Result<String, GBError> {
    let param = R16::new(opcode, 4, R16Type::R16Mem)?;
    let addr = param.read(&bus.registers);
    bus.write(addr, bus.registers.a)?;
    let raw_param = alu::read_bits(opcode, 4, 2);
    if raw_param == 0x2 {
        param.write(addr + 1, &mut bus.registers);
    } else if raw_param == 0x3 {
        param.write(addr - 1, &mut bus.registers);
    }
    Ok(format!("ld [{}] a", param.log()))
}

pub fn load_a_r16mem(opcode: u8, bus: &mut Bus) -> Result<String, GBError> {
    let param = R16::new(opcode, 4, R16Type::R16Mem)?;
    let addr = param.read(&bus.registers);
    let value = bus.read(addr)?;
    bus.registers.a = value;
    let raw_param = alu::read_bits(opcode, 4, 2);
    if raw_param == 0x2 {
        param.write(addr + 1, &mut bus.registers);
    } else if raw_param == 0x3 {
        param.write(addr - 1, &mut bus.registers);
    }
    Ok(format!("ld a [{}]", param.log()))
}

pub fn ld_n16_sp(bus: &mut Bus) -> Result<String, GBError> {
    let addr = alu::read_u16(&bus.fetch(), &bus.fetch());
    let lsb = (bus.registers.sp & 0xFF) as u8;
    let msb = (bus.registers.sp >> 8) as u8;
    bus.write(addr, lsb)?;
    bus.write(addr + 1, msb)?;
    Ok(format!("ld [{:#X}] sp", addr))
}

pub fn ld_hl_sp_delta(bus: &mut Bus) -> Result<String, GBError> {
    let delta = bus.fetch() as i8;
    let result = (bus.registers.sp as i16).wrapping_add(delta as i16);
    alu::write_u16(&mut bus.registers.l, &mut bus.registers.h, result as u16);
    // HACK: The flag calculation for this instruction is really weird, this implementation is based
    // on the open-source emulator mGBA, hopefully it's fine :>
    let carry = (bus.registers.sp & 0xFF) + (delta as u16 & 0xFF) > 0xFF;
    let half_carry = (bus.registers.sp as u8 & 0xF) + (delta as u8 & 0xF) > 0xF;
    bus.tick();
    bus.registers
        .set_all_flags(&[0, 0, half_carry as u8, carry as u8])?;
    Ok(format!("ld hl sp+{}", delta))
}

pub fn ld_n16_a(bus: &mut Bus) -> Result<String, GBError> {
    let addr = alu::read_u16(&bus.fetch(), &bus.fetch());
    bus.write(addr, bus.registers.a)?;
    Ok(format!("ld [{:#X}] a", addr))
}

pub fn ld_a_n16(bus: &mut Bus) -> Result<String, GBError> {
    let addr = alu::read_u16(&bus.fetch(), &bus.fetch());
    bus.registers.a = bus.read(addr)?;
    Ok(format!("ld a [{:#X}]", addr))
}

pub fn push(opcode: u8, bus: &mut Bus) -> Result<String, GBError> {
    let r16_param = R16::new(opcode, 4, R16Type::R16Stk)?;
    let (msb, lsb) = r16_param.read_as_tuple(&bus.registers);
    bus.tick();
    bus.registers.sp -= 1;
    bus.write(bus.registers.sp, msb)?;
    bus.registers.sp -= 1;
    bus.write(bus.registers.sp, lsb)?;
    Ok(format!("push {}", r16_param.log()))
}

pub fn pop(opcode: u8, bus: &mut Bus) -> Result<String, GBError> {
    let r16_param = R16::new(opcode, 4, R16Type::R16Stk)?;
    let lsb = bus.read(bus.registers.sp)? as u16;
    bus.registers.sp = bus.registers.sp.wrapping_add(1);
    let msb = bus.read(bus.registers.sp)? as u16;
    bus.registers.sp = bus.registers.sp.wrapping_add(1);
    r16_param.write((msb << 8) | lsb, &mut bus.registers);
    bus.registers.f &= 0xF0;
    Ok(format!("pop {}", r16_param.log()))
}
