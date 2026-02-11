use crate::{
    cpu::{
        alu,
        cpu_context::CpuContext,
        operands::{R8, R16, R16Type},
    },
    error::GBError,
};

pub fn load8(context: &mut CpuContext, opcode: u8) -> Result<(), GBError> {
    print!("ld ");
    let src_param = R8::get_r8_param(alu::read_bits(opcode, 6, 1) == 0, opcode, 0, context);
    let src = src_param.read(context)?;
    let dst_param = R8::get_r8_param(false, opcode, 3, context);
    dst_param.log();
    src_param.log();
    dst_param.write(context, src)?;
    Ok(())
}

pub fn load16(context: &mut CpuContext, opcode: u8) -> Result<(), GBError> {
    let param = R16::new(opcode, 4, R16Type::R16)?;
    param.write(
        alu::read_u16(&context.fetch(), &context.fetch()),
        &mut context.registers,
    );
    print!("ld r16 imm16");
    Ok(())
}

pub fn load_r16mem_a(opcode: u8, context: &mut CpuContext) -> Result<(), GBError> {
    let param = R16::new(opcode, 4, R16Type::R16Mem)?;
    let addr = param.read(&context.registers);
    context
        .memory
        .write(&mut context.clock, addr, context.registers.a)?;
    let raw_param = alu::read_bits(opcode, 4, 2);
    if raw_param == 0x2 {
        param.write(addr + 1, &mut context.registers);
    } else if raw_param == 0x3 {
        param.write(addr - 1, &mut context.registers);
    }
    print!("ld [r16mem] a");
    Ok(())
}

pub fn load_a_r16mem(opcode: u8, context: &mut CpuContext) -> Result<(), GBError> {
    let param = R16::new(opcode, 4, R16Type::R16Mem)?;
    let addr = param.read(&context.registers);
    let value = context.memory.read(&mut context.clock, addr)?;
    context.registers.a = value;
    let raw_param = alu::read_bits(opcode, 4, 2);
    if raw_param == 0x2 {
        param.write(addr + 1, &mut context.registers);
    } else if raw_param == 0x3 {
        param.write(addr - 1, &mut context.registers);
    }
    print!("ld a [r16mem]");
    Ok(())
}

pub fn ld_n16_sp(context: &mut CpuContext) -> Result<(), GBError> {
    print!("ld [n16] sp");
    let addr = alu::read_u16(&context.fetch(), &context.fetch());
    let lsb = (context.registers.sp & 0xFF) as u8;
    let msb = (context.registers.sp >> 8) as u8;
    context.memory.write(&mut context.clock, addr, lsb)?;
    context.memory.write(&mut context.clock, addr + 1, msb)?;
    Ok(())
}

pub fn ld_hl_sp_delta(context: &mut CpuContext) -> Result<(), GBError> {
    print!("ld hl sp+e8");
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
    context.clock.tick();
    context
        .registers
        .set_all_flags(&[0, 0, half_carry as u8, carry as u8])?;
    Ok(())
}

pub fn push(opcode: u8, context: &mut CpuContext) -> Result<(), GBError> {
    print!("push r16");
    let r16_param = R16::new(opcode, 4, R16Type::R16Stk)?;
    let (msb, lsb) = r16_param.read_as_tuple(&context.registers);
    context.clock.tick();
    context.registers.sp -= 1;
    context
        .memory
        .write(&mut context.clock, context.registers.sp, msb)?;
    context.registers.sp -= 1;
    context
        .memory
        .write(&mut context.clock, context.registers.sp, lsb)?;
    Ok(())
}
