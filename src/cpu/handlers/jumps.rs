use crate::{
    cpu::{alu, cpu_context::CpuContext},
    error::GBError,
};

pub fn jmp(context: &mut CpuContext, opcode: u8, is_relative: bool) -> Result<(), GBError> {
    let target_address: u16;
    let is_conditional: bool;
    if is_relative {
        print!("jr ");
        is_conditional = opcode != 0x18;
        let delta = context.fetch() as i8;
        target_address = (context.registers.pc as i16 + delta as i16) as u16;
    } else {
        print!("jp ");
        is_conditional = opcode != 0xC3;
        target_address = alu::read_u16(&context.fetch(), &context.fetch());
    }
    if is_conditional {
        print!("cc ");
    }
    print!("n16");
    if context
        .registers
        .match_condition(alu::read_bits(opcode, 3, 2))?
        || !is_conditional
    {
        context.registers.pc = target_address;
        context.clock.tick(&mut context.memory.io[0x44]);
    }
    Ok(())
}

pub fn call(context: &mut CpuContext, opcode: u8) -> Result<(), GBError> {
    let target_address: u16 = alu::read_u16(&context.fetch(), &context.fetch());
    let is_conditional: bool = opcode != 0xC3;
    print!("call ");
    if is_conditional {
        print!("cc ");
    }
    print!("n16");
    if context
        .registers
        .match_condition(alu::read_bits(opcode, 3, 2))?
        || !is_conditional
    {
        context.registers.sp -= 1;
        context.memory.write(
            &mut context.clock,
            context.registers.sp,
            (context.registers.pc >> 8) as u8,
        )?;
        context.registers.sp -= 1;
        context.memory.write(
            &mut context.clock,
            context.registers.sp,
            (context.registers.pc & 0xFF) as u8,
        )?;
        context.registers.pc = target_address;
        context.clock.tick(&mut context.memory.io[0x44]);
    }
    Ok(())
}

pub fn ret(context: &mut CpuContext, opcode: u8) -> Result<(), GBError> {
    print!("ret");
    if opcode != 0xC9 || opcode != 0xD9 {
        print!(" cc");
        context.clock.tick(&mut context.memory.io[0x44]);
        if !context
            .registers
            .match_condition(alu::read_bits(opcode, 3, 2))?
        {
            return Ok(());
        }
    }
    let lsb = context
        .memory
        .read(&mut context.clock, context.registers.sp)?;
    context.registers.sp += 1;
    let msb = context
        .memory
        .read(&mut context.clock, context.registers.sp)?;
    context.registers.sp += 1;
    let addr = alu::read_u16(&lsb, &msb);
    if opcode == 0xD9 {
        context.memory.ie = 1;
        print!("i");
    }
    context.clock.tick(&mut context.memory.io[0x44]);
    context.registers.pc = addr;
    Ok(())
}
