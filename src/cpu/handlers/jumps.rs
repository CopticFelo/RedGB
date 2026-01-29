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
        context.clock.tick();
    }
    Ok(())
}
