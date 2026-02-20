use std::slice;

use crate::{
    cpu::{alu, clock::Clock, handlers::*, reg_file::RegFile},
    error::GBError,
    mem::map::MemoryMap,
};

pub struct CpuContext {
    pub registers: RegFile,
    pub memory: MemoryMap,
    pub clock: Clock,
}

impl CpuContext {
    pub fn init(registers: RegFile, memory: MemoryMap, clock: Clock) -> Self {
        Self {
            registers,
            memory,
            clock,
        }
    }

    pub fn fetch(&mut self) -> u8 {
        let result = match self.memory.read(&mut self.clock, self.registers.pc) {
            Ok(op) => op,
            // HACK: Probably improper error handling
            Err(s) => {
                println!("{}", s);
                0x0
            }
        };
        self.registers.pc += 1;
        result
    }

    pub fn start_exec_cycle(&mut self) -> Result<(), GBError> {
        loop {
            let opcode = self.fetch();
            print!("{:#X}: ", self.registers.pc);
            print!("{:#X} -> ", opcode);
            match opcode {
                0x0 => print!("nop"), // NOP
                0xF3 => {
                    print!("DI");
                    self.memory.ie = 0;
                } // DI
                0xFB => {
                    print!("EI");
                    self.memory.ie = 1;
                } // EI
                0xC2 | 0xD2 | 0xCA | 0xDA | 0xC3 => jumps::jmp(self, opcode, false)?, // JP cc, imm16 | JP imm16
                0x20 | 0x30 | 0x28 | 0x38 | 0x18 => jumps::jmp(self, opcode, true)?, // JR cc, imm8 | JR imm8
                0xC4 | 0xD4 | 0xCC | 0xDC | 0xCD => jumps::call(self, opcode)?, // CALL imm16 | CALL cc imm16
                0xD9 | 0xC9 | 0xD8 | 0xC8 | 0xD0 | 0xC0 => jumps::ret(self, opcode)?, // RET | RETI | RET cc
                0xC7 | 0xD7 | 0xE7 | 0xF7 | 0xCF | 0xDF | 0xEF | 0xFF => jumps::rst(self, opcode)?, // RST tgt3
                0xE9 => {
                    println!("jp [hl]");
                    self.registers.pc = alu::read_u16(&self.registers.l, &self.registers.h);
                } // JP hl
                0xF8 => loads_16::ld_hl_sp_delta(self)?, // LD HL SP+E8
                0xF9 => {
                    print!("ld sp hl");
                    self.registers.sp = alu::read_u16(&self.registers.l, &self.registers.h);
                    self.clock.tick(&mut self.memory.io[0x44]);
                } // LD SP HL
                0xE0 => {
                    print!("ldh [a8] a");
                    let addr = 0xFF00 + self.fetch() as u16;
                    self.memory.write(&mut self.clock, addr, self.registers.a)?;
                } // LDH [A8] A
                0xF0 => {
                    print!("ldh a [a8]");
                    let addr = 0xFF00 + self.fetch() as u16;
                    self.registers.a = self.memory.read(&mut self.clock, addr)?;
                } // LDH A [A8]
                0xE2 => {
                    print!("ldh [C] a");
                    let addr = 0xFF00 + self.registers.c as u16;
                    self.memory.write(&mut self.clock, addr, self.registers.a)?;
                } // LDH [C] A
                0xF2 => {
                    print!("ldh a [C]");
                    let addr = 0xFF00 + self.registers.c as u16;
                    self.registers.a = self.memory.read(&mut self.clock, addr)?;
                } // LDH A [C]
                0x8 => loads_16::ld_n16_sp(self)?,       // LD [imm16] SP
                0x06 | 0x16 | 0x26 | 0x36 | 0x0E | 0x1E | 0x2E | 0x3E | 0x40..0x80 => {
                    loads::load_r8(self, opcode)?
                } // LD r8, r8 | LD r8, [hl] | LD [hl], r8
                0xEA => loads_16::ld_n16_a(self)?,       // LD [imm16] A
                0xFA => loads_16::ld_a_n16(self)?,       // LD A [imm16]
                0x01 | 0x11 | 0x21 | 0x31 => loads_16::load_r16_imm16(self, opcode)?, // LD r16, imm16
                0x02 | 0x12 | 0x22 | 0x32 => loads_16::load_r16mem_a(opcode, self)?, // LD [r16mem] A
                0x0A | 0x1A | 0x2A | 0x3A => loads_16::load_a_r16mem(opcode, self)?, // LD A, [r16mem]
                0x80..0x90 | 0xC6 | 0xCE => arithmetic::add(opcode, self)?, // ADD/ADC A, r8 | ADD/ADC A, [hl] | ADD/ADC A, imm8
                0x90..0xA0 | 0xD6 | 0xDE => arithmetic::sub(opcode, self)?, // SUB/SBC A, r8 | SUB/SBC A, [hl] | SUB/SBC A, imm8
                0xA0..0xA8 | 0xE6 => arithmetic::and(opcode, self)?, // AND A, r8 | AND A, [hl] | AND A, imm8
                0xA8..0xB0 | 0xEE => arithmetic::xor(opcode, self)?, // XOR A, r8 | XOR A, [hl] | XOR A, imm8
                0xB0..0xB8 | 0xF6 => arithmetic::or(opcode, self)?, // OR A, r8 | OR A, [hl] | OR A, imm8
                0xB8..0xC0 | 0xFE => arithmetic::cp(opcode, self)?, // CP A, r8 | CP A, [hl] | CP A, imm8
                0xC1 | 0xD1 | 0xE1 | 0xF1 => loads_16::pop(opcode, self)?, // POP R16
                0xC5 | 0xD5 | 0xE5 | 0xF5 => loads_16::push(opcode, self)?, // PUSH R16
                0x03 | 0x13 | 0x23 | 0x33 => arithmetic_16::inc_r16(opcode, self, 1)?, // INC R16
                0x0B | 0x1B | 0x2B | 0x3B => arithmetic_16::inc_r16(opcode, self, -1)?, // DEC R16
                0x09 | 0x19 | 0x29 | 0x39 => arithmetic_16::add_hl(opcode, self)?, // ADD HL R16
                0xE8 => arithmetic_16::add_sp_delta(self)?,         // ADD SP, SP+E8
                0x04 | 0x14 | 0x24 | 0x34 | 0x0C | 0x1C | 0x2C | 0x3C => {
                    arithmetic::inc_r8(opcode, self, 1)?
                } // INC r8, INC [hl]
                0x05 | 0x15 | 0x25 | 0x35 | 0x0D | 0x1D | 0x2D | 0x3D => {
                    arithmetic::inc_r8(opcode, self, -1)?
                } // DEC r8, DEC [hl]
                0x07 | 0x17 => {
                    let through_carry = alu::read_bits(opcode, 4, 1) == 1;
                    print!("{} ", if through_carry { "rla" } else { "rlca" });
                    let (a, carry) = alu::rotate_left(
                        self.registers.a,
                        self.registers.read_flag(crate::cpu::reg_file::Flag::Carry),
                        through_carry,
                    );
                    self.registers.a = a;
                    self.registers.set_all_flags(&[0, 0, 0, carry as u8])?;
                } // RLA | RLCA
                0x0F | 0x1F => {
                    let through_carry = alu::read_bits(opcode, 4, 1) == 1;
                    print!("{} ", if through_carry { "rra" } else { "rrca" });
                    let (a, carry) = alu::rotate_right(
                        self.registers.a,
                        self.registers.read_flag(crate::cpu::reg_file::Flag::Carry),
                        through_carry,
                    );
                    self.registers.a = a;
                    self.registers.set_all_flags(&[0, 0, 0, carry as u8])?;
                } // RRA | RRCA
                // TODO: It's probably a good idea to merge these 2 branches above ^
                0xCB => self.prefixed_instr()?,
                0xD3 | 0xDB | 0xDD | 0xE3 | 0xE4 | 0xEB..0xEE | 0xF4 | 0xFC | 0xFD => {
                    return Err(GBError::IllegalInstruction(opcode));
                }
                _ => print!("<unsupported>"),
            }
            println!();
        }
    }

    fn prefixed_instr(&mut self) -> Result<(), GBError> {
        let opcode = self.fetch();
        match opcode {
            0x0..0x20 => bitwise::rotate_to_carry(opcode, self), // RL R8 | RR R8 | RLC R8 | RRC R8
            0x20..0x30 | 0x38..0x40 => bitwise::shift_to_carry(opcode, self), // SRL R8 | SLA R8 | SRA R8
            0x30..0x38 => bitwise::swap(opcode, self),                        // SWAP R8
            0x40..0x80 => bitwise::test_bit(opcode, self),                    // BIT U3 R8
            0x80..0xC0 => bitwise::reset_bit(opcode, self),                   // RES U3 R8
            0xC0..=0xFF => bitwise::set_bit(opcode, self),                    // SET U3 R8
        }
    }
}
