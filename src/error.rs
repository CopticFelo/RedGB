use thiserror::Error;

#[derive(Error, Debug)]
pub enum GBError {
    #[error("Memory Address {0:#X} is invalid")]
    BadAddress(u16),
    #[error("Memory Address {0:#X} is Read-only")]
    ReadOnlyAddress(u16),
    #[error("Memory Address {0:#X} is prohibited")]
    IllegalAddress(u16),
    #[error("Invalid R8 operand {0:#X}")]
    InvalidR8Operand(u8),
    #[error("Invalid R16 operand {0:#X}")]
    InvalidR16Operand(u8),
    #[error("Invalid Condition operand {0:#X}")]
    InvalidCondition(u8),
    #[error("Illegal Instruction {0:#X}")]
    IllegalInstruction(u8),
    #[error("Error: Trying to insert {length} bits at index {index} (Overflow)")]
    ByteOverflow { length: u8, index: u8 },
}
