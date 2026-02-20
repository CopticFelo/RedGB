use crate::{
    cpu::{alu, cpu_context::CpuContext, operands::R8},
    error::GBError,
};

pub fn rotate_to_carry(opcode: u8, context: &mut CpuContext) -> Result<(), GBError> {
    let is_left = alu::read_bits(opcode, 3, 1) == 0;
    let through_carry = alu::read_bits(opcode, 4, 1) == 1;
    let r8_param = R8::get_r8_param(false, opcode, 0, context);
    let r8 = r8_param.read(context)?;
    let (result, carry) = if is_left {
        print!("{} ", if through_carry { "rl" } else { "rlc" });
        alu::rotate_left(
            r8,
            context
                .registers
                .read_flag(crate::cpu::reg_file::Flag::Carry),
            through_carry,
        )
    } else {
        print!("{} ", if through_carry { "rr" } else { "rrc" });
        alu::rotate_right(
            r8,
            context
                .registers
                .read_flag(crate::cpu::reg_file::Flag::Carry),
            through_carry,
        )
    };
    r8_param.log();
    r8_param.write(context, result)?;
    context
        .registers
        .set_all_flags(&[(result == 0) as u8, 0, 0, carry as u8])?;
    Ok(())
}

pub fn shift_to_carry(opcode: u8, context: &mut CpuContext) -> Result<(), GBError> {
    let is_left = alu::read_bits(opcode, 3, 1) == 0;
    let is_logical = alu::read_bits(opcode, 4, 1) == 1;
    let r8_param = R8::get_r8_param(false, opcode, 0, context);
    let r8 = r8_param.read(context)?;
    let (result, carry) = if is_logical {
        print!("srl ");
        let c = alu::read_bits(r8, 0, 1);
        let res = r8 >> 1;
        (res, c)
    } else if is_left {
        print!("sla ");
        let c = alu::read_bits(r8, 7, 1);
        let res = r8 << 1;
        (res, c)
    } else {
        print!("sra ");
        let c = alu::read_bits(r8, 0, 1);
        let last = alu::read_bits(r8, 7, 1);
        let res = (r8 >> 1) | (last << 7);
        (res, c)
    };
    r8_param.log();
    r8_param.write(context, result)?;
    context
        .registers
        .set_all_flags(&[(result == 0) as u8, 0, 0, carry])?;
    Ok(())
}
