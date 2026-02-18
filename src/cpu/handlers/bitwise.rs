use crate::{
    cpu::{alu, cpu_context::CpuContext, operands::R8},
    error::GBError,
};

pub fn rotate_to_carry(opcode: u8, context: &mut CpuContext) -> Result<(), GBError> {
    let is_left = alu::read_bits(opcode, 3, 1) == 0;
    let r8_param = R8::get_r8_param(false, opcode, 0, context);
    let r8 = r8_param.read(context)?;
    let (result, carry) = if is_left {
        print!("rl ");
        let last = alu::read_bits(r8, 7, 1);
        let res = r8 << 1;
        (res, last)
    } else {
        print!("rr ");
        let first = alu::read_bits(r8, 0, 1);
        let res = r8 >> 1;
        (res, first)
    };
    r8_param.log();
    r8_param.write(context, result)?;
    context
        .registers
        .set_all_flags(&[(result == 0) as u8, 0, 0, carry])?;
    Ok(())
}
