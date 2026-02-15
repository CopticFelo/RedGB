use crate::{
    cpu::{
        cpu_context::{self, CpuContext},
        operands::{R16, R16Type},
    },
    error::GBError,
};

pub fn inc_r16(opcode: u8, context: &mut CpuContext, delta: i8) -> Result<(), GBError> {
    let r16_param = R16::new(opcode, 4, R16Type::R16)?;
    let r16 = r16_param.read(&context.registers);
    let result = r16 as i16 + delta as i16;
    r16_param.write(result as u16, &mut context.registers);
    context.clock.tick();
    Ok(())
}
