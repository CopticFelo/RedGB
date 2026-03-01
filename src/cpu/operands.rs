use crate::{
    cpu::{alu, cpu_context::CpuContext, reg_file::RegFile},
    error::GBError,
    mem::map::MemoryMap,
};

const REG_NAMES: [&str; 8] = ["b", "c", "d", "e", "h", "l", "", "a"];

// the r8 param is a 3 bit param in the instruction opcode
// it represents an 8-bit register
// or the memory value (8-bit) pointed to by the 16-bit hl register
// from 0-7 in order (b,c,d,e,h,l,[hl],a)
pub enum R8 {
    Register(u8),
    Hl(u16),
    N8(u8), // this is added for convinience some instructions that take r8 have an identical
            // version that takes imm8 (i.e the next byte on the rom)
}

impl R8 {
    pub fn get_r8_param(n8: bool, opcode: u8, index: u8, context: &mut CpuContext) -> Self {
        if n8 {
            return Self::N8(context.fetch());
        }
        let param = alu::read_bits(opcode, index, 3);
        if param == 6 {
            let addr = alu::read_u16(&context.registers.l, &context.registers.h);
            Self::Hl(addr)
        } else {
            Self::Register(param)
        }
    }

    pub fn read(&self, context: &mut CpuContext) -> Result<u8, GBError> {
        match self {
            Self::Register(reg) => Ok(*context.registers.match_r8(*reg)?),
            Self::Hl(addr) => Ok(MemoryMap::read(context, *addr)?),
            Self::N8(n) => Ok(*n),
        }
    }

    pub fn write(&self, context: &mut CpuContext, value: u8) -> Result<(), GBError> {
        match self {
            Self::Register(reg) => {
                *context.registers.match_r8(*reg)? = value;
                Ok(())
            }
            Self::Hl(addr) => {
                MemoryMap::write(context, *addr, value)?;
                Ok(())
            }
            Self::N8(_) => Ok(()),
        }
    }

    pub fn log(&self) -> String {
        match self {
            Self::Register(reg) => format!("{}", REG_NAMES[*reg as usize]),
            Self::Hl(addr) => format!("[{:#X}]", addr),
            Self::N8(n8) => format!("{:#X}", n8),
        }
    }
}

pub enum R16Type {
    R16,
    R16Stk,
    R16Mem,
}

#[derive(Clone, Copy, Debug)]
pub enum R16 {
    BC,
    DE,
    HL,
    AF,
    SP,
}

impl R16 {
    pub fn new(opcode: u8, index: u8, r16type: R16Type) -> Result<Self, GBError> {
        let param = alu::read_bits(opcode, index, 2);
        match param {
            0x0 => Ok(Self::BC),
            0x1 => Ok(Self::DE),
            0x2 => Ok(Self::HL),
            0x3 => match r16type {
                R16Type::R16 => Ok(Self::SP),
                R16Type::R16Stk => Ok(Self::AF),
                R16Type::R16Mem => Ok(Self::HL),
            },
            _ => Err(GBError::InvalidR16Operand(param)),
        }
    }

    pub fn read(&self, reg_file: &RegFile) -> u16 {
        match self {
            R16::BC => alu::read_u16(&reg_file.c, &reg_file.b),
            R16::DE => alu::read_u16(&reg_file.e, &reg_file.d),
            R16::HL => alu::read_u16(&reg_file.l, &reg_file.h),
            R16::AF => alu::read_u16(&reg_file.f, &reg_file.a),
            R16::SP => reg_file.sp,
        }
    }

    pub fn read_as_tuple(&self, reg_file: &RegFile) -> (u8, u8) {
        match self {
            R16::BC => (reg_file.b, reg_file.c),
            R16::DE => (reg_file.d, reg_file.e),
            R16::HL => (reg_file.h, reg_file.l),
            R16::AF => (reg_file.a, reg_file.f),
            R16::SP => ((reg_file.sp >> 8) as u8, (reg_file.sp & 0xFF) as u8),
        }
    }

    pub fn write(&self, value: u16, reg_file: &mut RegFile) {
        match self {
            R16::BC => alu::write_u16(&mut reg_file.c, &mut reg_file.b, value),
            R16::DE => alu::write_u16(&mut reg_file.e, &mut reg_file.d, value),
            R16::HL => alu::write_u16(&mut reg_file.l, &mut reg_file.h, value),
            R16::AF => alu::write_u16(&mut reg_file.f, &mut reg_file.a, value),
            R16::SP => reg_file.sp = value,
        }
    }

    pub fn log(&self) -> String {
        format!("{:?}", *self)
    }
}
