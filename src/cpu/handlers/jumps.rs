use crate::{
    cpu::{alu, cpu_context::CpuContext},
    error::GBError,
    mem::map::MemoryMap,
};

const CONDITION_NAMES: [&str; 4] = ["nz", "z", "nc", "c"];

pub fn jmp(context: &mut CpuContext, opcode: u8, is_relative: bool) -> Result<String, GBError> {
    let target_address: u16;
    let is_conditional: bool;
    let mut log: String = String::new();
    if is_relative {
        log += "jr ";
        is_conditional = opcode != 0x18;
        let delta = context.fetch() as i8;
        target_address = (context.registers.pc as i16 + delta as i16) as u16;
    } else {
        log += "jp ";
        is_conditional = opcode != 0xC3;
        target_address = alu::read_u16(&context.fetch(), &context.fetch());
    }
    let condition = alu::read_bits(opcode, 3, 2);
    if is_conditional {
        log += CONDITION_NAMES[condition as usize];
    }
    log += &format!(" {:#X}", target_address);
    if context.registers.match_condition(condition)? || !is_conditional {
        context.registers.pc = target_address;
        context.tick();
    }
    Ok(log)
}

pub fn call(context: &mut CpuContext, opcode: u8) -> Result<String, GBError> {
    let target_address: u16 = alu::read_u16(&context.fetch(), &context.fetch());
    let is_conditional: bool = opcode != 0xC3;
    let mut log = String::from("call ");
    let condition = alu::read_bits(opcode, 3, 2);
    if is_conditional {
        log += CONDITION_NAMES[condition as usize];
    }
    log += &format!(" {:#X}", target_address);
    if context.registers.match_condition(condition)? || !is_conditional {
        context.registers.sp -= 1;
        MemoryMap::write(
            context,
            context.registers.sp,
            (context.registers.pc >> 8) as u8,
        )?;
        context.registers.sp -= 1;
        MemoryMap::write(
            context,
            context.registers.sp,
            (context.registers.pc & 0xFF) as u8,
        )?;
        context.registers.pc = target_address;
        context.tick();
    }
    Ok(log)
}

pub fn ret(context: &mut CpuContext, opcode: u8) -> Result<String, GBError> {
    let mut log = String::from("ret");
    if opcode == 0xD9 {
        context.memory.ie = 1;
        log += "i";
    }
    if opcode != 0xC9 || opcode != 0xD9 {
        let condition = alu::read_bits(opcode, 3, 2);
        log += &format!(" {}", CONDITION_NAMES[condition as usize]);
        context.tick();
        if !context
            .registers
            .match_condition(alu::read_bits(opcode, 3, 2))?
        {
            return Ok(log);
        }
    }
    let lsb = MemoryMap::read(context, context.registers.sp)?;
    context.registers.sp += 1;
    let msb = MemoryMap::read(context, context.registers.sp)?;
    context.registers.sp += 1;
    let addr = alu::read_u16(&lsb, &msb);
    context.tick();
    context.registers.pc = addr;
    Ok(log)
}

pub fn rst(context: &mut CpuContext, opcode: u8) -> Result<String, GBError> {
    let addr = (alu::read_bits(opcode, 3, 3) * 8) as u16;
    context.tick();
    context.registers.sp -= 1;
    MemoryMap::write(
        context,
        context.registers.sp,
        (context.registers.pc >> 8) as u8,
    )?;
    context.registers.sp -= 1;
    MemoryMap::write(
        context,
        context.registers.sp,
        (context.registers.pc & 0xFF) as u8,
    )?;
    context.registers.pc = addr;
    Ok(format!("rst {:#X}", addr))
}
