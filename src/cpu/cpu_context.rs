use std::slice;

use crate::{
    cpu::{
        alu,
        clock::Clock,
        handlers::{loads::ld_hl_sp_delta, *},
        reg_file::RegFile,
    },
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
                0x0 => print!("nop"),                                                 // NOP
                0xC2 | 0xD2 | 0xCA | 0xDA | 0xC3 => jumps::jmp(self, opcode, false)?, // JP cc, imm16 | JP imm16
                0x20 | 0x30 | 0x28 | 0x38 | 0x18 => jumps::jmp(self, opcode, true)?, // JR cc, imm8 | JR imm8
                0xE9 => {
                    println!("jp [hl]");
                    self.registers.pc = alu::read_u16(&self.registers.l, &self.registers.h);
                } // JP hl
                0xF8 => ld_hl_sp_delta(self)?,                                       // LD HL SP+E8
                0xF9 => {
                    print!("ld sp hl");
                    self.registers.sp = alu::read_u16(&self.registers.l, &self.registers.h);
                    self.clock.tick();
                } // LD SP HL
                0x8 => loads::ld_n16_sp(self)?, // LD [imm16] SP
                0x06 | 0x16 | 0x26 | 0x36 | 0x0E | 0x1E | 0x2E | 0x3E | 0x40..0x80 => {
                    loads::load8(self, opcode)?
                } // LD r8, r8 | LD r8, [hl] | LD [hl], r8
                0x01 | 0x11 | 0x21 | 0x31 => loads::load16(self, opcode)?, // LD r16, imm16
                0x02 | 0x12 | 0x22 | 0x32 => loads::load_r16mem_a(opcode, self)?, // LD [r16mem] A
                0x0A | 0x1A | 0x2A | 0x3A => loads::load_a_r16mem(opcode, self)?, // LD A, [r16mem]
                0x80..0x90 | 0xC6 | 0xCE => arithmetic::add(opcode, self)?, // ADD/ADC A, r8 | ADD/ADC A, [hl] | ADD/ADC A, imm8
                0x90..0xA0 | 0xD6 | 0xDE => arithmetic::sub(opcode, self)?, // SUB/SBC A, r8 | SUB/SBC A, [hl] | SUB/SBC A, imm8
                0xA0..0xA8 | 0xE6 => arithmetic::and(opcode, self)?, // AND A, r8 | AND A, [hl] | AND A, imm8
                0xA8..0xB0 | 0xEE => arithmetic::xor(opcode, self)?, // XOR A, r8 | XOR A, [hl] | XOR A, imm8
                0xB0..0xB8 | 0xF6 => arithmetic::or(opcode, self)?, // OR A, r8 | OR A, [hl] | OR A, imm8
                0xB8..0xC0 | 0xFE => arithmetic::cp(opcode, self)?, // CP A, r8 | CP A, [hl] | CP A, imm8
                0x04 | 0x14 | 0x24 | 0x34 | 0x0C | 0x1C | 0x2C | 0x3C => {
                    arithmetic::inc_r8(opcode, self, 1)?
                } // INC r8, INC [hl]
                0x05 | 0x15 | 0x25 | 0x35 | 0x0D | 0x1D | 0x2D | 0x3D => {
                    arithmetic::inc_r8(opcode, self, -1)?
                } // DEC r8, DEC [hl]
                0xC5 | 0xD5 | 0xE5 | 0xF5 => loads::push(opcode, self)?,
                0xD3 | 0xDB | 0xDD | 0xE3 | 0xE4 | 0xEB..0xEE | 0xF4 | 0xFC | 0xFD => {
                    return Err(GBError::IllegalInstruction(opcode));
                }
                _ => print!("<unsupported>"),
            }
            println!();
        }
    }
}
