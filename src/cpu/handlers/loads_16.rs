use crate::{
    cpu::{
        alu,
        cpu_context::CpuContext,
        operands::{R16, R16Type},
    },
    error::GBError,
    mem::map::MemoryMap,
};

pub fn load_r16_imm16(context: &mut CpuContext, opcode: u8) -> Result<String, GBError> {
    let param = R16::new(opcode, 4, R16Type::R16)?;
    let value = alu::read_u16(&context.fetch(), &context.fetch());
    param.write(value, &mut context.registers);
    Ok(format!("ld {} {:#X}", param.log(), value))
}

pub fn load_r16mem_a(opcode: u8, context: &mut CpuContext) -> Result<String, GBError> {
    let param = R16::new(opcode, 4, R16Type::R16Mem)?;
    let addr = param.read(&context.registers);
    MemoryMap::write(context, addr, context.registers.a)?;
    let raw_param = alu::read_bits(opcode, 4, 2);
    if raw_param == 0x2 {
        param.write(addr + 1, &mut context.registers);
    } else if raw_param == 0x3 {
        param.write(addr - 1, &mut context.registers);
    }
    Ok(format!("ld [{}] a", param.log()))
}

pub fn load_a_r16mem(opcode: u8, context: &mut CpuContext) -> Result<String, GBError> {
    let param = R16::new(opcode, 4, R16Type::R16Mem)?;
    let addr = param.read(&context.registers);
    let value = MemoryMap::read(context, addr)?;
    context.registers.a = value;
    let raw_param = alu::read_bits(opcode, 4, 2);
    if raw_param == 0x2 {
        param.write(addr + 1, &mut context.registers);
    } else if raw_param == 0x3 {
        param.write(addr - 1, &mut context.registers);
    }
    Ok(format!("ld a [{}]", param.log()))
}

pub fn ld_n16_sp(context: &mut CpuContext) -> Result<String, GBError> {
    let addr = alu::read_u16(&context.fetch(), &context.fetch());
    let lsb = (context.registers.sp & 0xFF) as u8;
    let msb = (context.registers.sp >> 8) as u8;
    MemoryMap::write(context, addr, lsb)?;
    MemoryMap::write(context, addr + 1, msb)?;
    Ok(format!("ld [{:#X}] sp", addr))
}

pub fn ld_hl_sp_delta(context: &mut CpuContext) -> Result<String, GBError> {
    let delta = context.fetch() as i8;
    let result = (context.registers.sp as i16) + delta as i16;
    alu::write_u16(
        &mut context.registers.l,
        &mut context.registers.h,
        result as u16,
    );
    // HACK: The flag calculation for this instruction is really weird, this implementation is based
    // on the open-source emulator mGBA, hopefully it's fine :>
    let carry = (context.registers.sp & 0xFF) + (delta as u16 & 0xFF) > 0xFF;
    let half_carry = (context.registers.sp as u8 & 0xF) + (delta as u8 & 0xF) > 0xF;
    context.tick();
    context
        .registers
        .set_all_flags(&[0, 0, half_carry as u8, carry as u8])?;
    Ok(format!("ld hl sp+{}", delta))
}

pub fn ld_n16_a(context: &mut CpuContext) -> Result<String, GBError> {
    let addr = alu::read_u16(&context.fetch(), &context.fetch());
    MemoryMap::write(context, addr, context.registers.a)?;
    Ok(format!("ld [{:#X}] a", addr))
}

pub fn ld_a_n16(context: &mut CpuContext) -> Result<String, GBError> {
    let addr = alu::read_u16(&context.fetch(), &context.fetch());
    context.registers.a = MemoryMap::read(context, addr)?;
    Ok(format!("ld a [{:#X}]", addr))
}

pub fn push(opcode: u8, context: &mut CpuContext) -> Result<String, GBError> {
    let r16_param = R16::new(opcode, 4, R16Type::R16Stk)?;
    let (msb, lsb) = r16_param.read_as_tuple(&context.registers);
    context.tick();
    context.registers.sp -= 1;
    MemoryMap::write(context, context.registers.sp, msb)?;
    context.registers.sp -= 1;
    MemoryMap::write(context, context.registers.sp, lsb)?;
    Ok(format!("push {}", r16_param.log()))
}

pub fn pop(opcode: u8, context: &mut CpuContext) -> Result<String, GBError> {
    let r16_param = R16::new(opcode, 4, R16Type::R16Stk)?;
    let lsb = MemoryMap::read(context, context.registers.sp)? as u16;
    context.registers.sp += 1;
    let msb = MemoryMap::read(context, context.registers.sp)? as u16;
    context.registers.sp += 1;
    r16_param.write((msb << 8) | lsb, &mut context.registers);
    Ok(format!("pop {}", r16_param.log()))
}
