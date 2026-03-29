use log::debug;
use ringbuf::traits::Observer;

use crate::{
    bus::Bus,
    cpu::{alu, handlers::*, reg_file::Flag},
    error::GBError,
};

pub struct SM83;

impl SM83 {
    pub fn step(bus: &mut Bus) -> Result<(), GBError> {
        bus.ppu.frame_flag = false;
        loop {
            if bus.ppu.frame_flag && bus.apu.buffer.occupied_len() > 1024 {
                break Ok(());
            }
            if !bus.registers.exec {
                bus.tick();
                Self::handle_interupts(bus)?;
                continue;
            }
            let opcode_addr = bus.registers.pc;
            let opcode = bus.fetch();
            let result = match opcode {
                0x0 => Ok("nop".to_string()), // NOP
                0xF3 => {
                    bus.registers.ime = false;
                    Ok("di".to_string())
                } // DI
                0xFB => {
                    bus.registers.ime = true;
                    Ok("ei".to_string())
                } // EI
                0xC2 | 0xD2 | 0xCA | 0xDA | 0xC3 => jumps::jmp(bus, opcode, false), // JP cc, imm16 | JP imm16
                0x20 | 0x30 | 0x28 | 0x38 | 0x18 => jumps::jmp(bus, opcode, true), // JR cc, imm8 | JR imm8
                0xC4 | 0xD4 | 0xCC | 0xDC | 0xCD => jumps::call(bus, opcode), // CALL imm16 | CALL cc imm16
                0xD9 | 0xC9 | 0xD8 | 0xC8 | 0xD0 | 0xC0 => jumps::ret(bus, opcode), // RET | RETI | RET cc
                0xC7 | 0xD7 | 0xE7 | 0xF7 | 0xCF | 0xDF | 0xEF | 0xFF => jumps::rst(bus, opcode), // RST tgt3
                0xE9 => {
                    bus.registers.pc = alu::read_u16(&bus.registers.l, &bus.registers.h);
                    Ok("jp [hl]".to_string())
                } // JP hl
                0xF8 => loads_16::ld_hl_sp_delta(bus), // LD HL SP+E8
                0xF9 => {
                    bus.registers.sp = alu::read_u16(&bus.registers.l, &bus.registers.h);
                    bus.tick();
                    Ok("ld sp hl".to_string())
                } // LD SP HL
                0xE0 => {
                    let addr = 0xFF00 + bus.fetch() as u16;
                    bus.write(addr, bus.registers.a)?;
                    Ok(format!("ldh [{:#X}] a", addr))
                } // LDH [A8] A
                0xF0 => {
                    let addr = 0xFF00 + bus.fetch() as u16;
                    bus.registers.a = bus.read(addr)?;
                    Ok(format!("ldh a [{:#X}]", addr))
                } // LDH A [A8]
                0xE2 => {
                    let addr = 0xFF00 + bus.registers.c as u16;
                    bus.write(addr, bus.registers.a)?;
                    Ok("ldh [C] a".to_string())
                } // LDH [C] A
                0xF2 => {
                    let addr = 0xFF00 + bus.registers.c as u16;
                    bus.registers.a = bus.read(addr)?;
                    Ok("ldh a [C]".to_string())
                } // LDH A [C]
                0x8 => loads_16::ld_n16_sp(bus),       // LD [imm16] SP
                0x76 => {
                    bus.registers.exec = false;
                    Ok("halt".to_string())
                }
                0x06 | 0x16 | 0x26 | 0x36 | 0x0E | 0x1E | 0x2E | 0x3E | 0x40..0x80 => {
                    loads::load_r8(bus, opcode)
                } // LD r8, r8 | LD r8, [hl] | LD [hl], r8
                0xEA => loads_16::ld_n16_a(bus), // LD [imm16] A
                0xFA => loads_16::ld_a_n16(bus), // LD A [imm16]
                0x01 | 0x11 | 0x21 | 0x31 => loads_16::load_r16_imm16(bus, opcode), // LD r16, imm16
                0x02 | 0x12 | 0x22 | 0x32 => loads_16::load_r16mem_a(opcode, bus), // LD [r16mem] A
                0x0A | 0x1A | 0x2A | 0x3A => loads_16::load_a_r16mem(opcode, bus), // LD A, [r16mem]
                0x80..0x90 | 0xC6 | 0xCE => arithmetic::add(opcode, bus), // ADD/ADC A, r8 | ADD/ADC A, [hl] | ADD/ADC A, imm8
                0x90..0xA0 | 0xD6 | 0xDE => arithmetic::sub(opcode, bus), // SUB/SBC A, r8 | SUB/SBC A, [hl] | SUB/SBC A, imm8
                0xA0..0xA8 | 0xE6 => arithmetic::and(opcode, bus), // AND A, r8 | AND A, [hl] | AND A, imm8
                0xA8..0xB0 | 0xEE => arithmetic::xor(opcode, bus), // XOR A, r8 | XOR A, [hl] | XOR A, imm8
                0xB0..0xB8 | 0xF6 => arithmetic::or(opcode, bus), // OR A, r8 | OR A, [hl] | OR A, imm8
                0xB8..0xC0 | 0xFE => arithmetic::cp(opcode, bus), // CP A, r8 | CP A, [hl] | CP A, imm8
                0xC1 | 0xD1 | 0xE1 | 0xF1 => loads_16::pop(opcode, bus), // POP R16
                0xC5 | 0xD5 | 0xE5 | 0xF5 => loads_16::push(opcode, bus), // PUSH R16
                0x03 | 0x13 | 0x23 | 0x33 => arithmetic_16::inc_r16(opcode, bus, 1), // INC R16
                0x0B | 0x1B | 0x2B | 0x3B => arithmetic_16::inc_r16(opcode, bus, -1), // DEC R16
                0x09 | 0x19 | 0x29 | 0x39 => arithmetic_16::add_hl(opcode, bus), // ADD HL R16
                0xE8 => arithmetic_16::add_sp_delta(bus),         // ADD SP, SP+E8
                0x04 | 0x14 | 0x24 | 0x34 | 0x0C | 0x1C | 0x2C | 0x3C => {
                    arithmetic::inc_r8(opcode, bus, 1)
                } // INC r8, INC [hl]
                0x05 | 0x15 | 0x25 | 0x35 | 0x0D | 0x1D | 0x2D | 0x3D => {
                    arithmetic::inc_r8(opcode, bus, -1)
                } // DEC r8, DEC [hl]
                0x07 | 0x17 => {
                    let through_carry = alu::read_bits(opcode, 4, 1) == 1;
                    let (a, carry) = alu::rotate_left(
                        bus.registers.a,
                        bus.registers.read_flag(crate::cpu::reg_file::Flag::Carry),
                        through_carry,
                    );
                    bus.registers.a = a;
                    bus.registers.set_all_flags(&[0, 0, 0, carry as u8])?;
                    Ok((if through_carry { "rla" } else { "rlca" }).to_string())
                } // RLA | RLCA
                0x0F | 0x1F => {
                    let through_carry = alu::read_bits(opcode, 4, 1) == 1;
                    let (a, carry) = alu::rotate_right(
                        bus.registers.a,
                        bus.registers.read_flag(crate::cpu::reg_file::Flag::Carry),
                        through_carry,
                    );
                    bus.registers.a = a;
                    bus.registers.set_all_flags(&[0, 0, 0, carry as u8])?;
                    Ok(format!("{} ", if through_carry { "rra" } else { "rrca" }))
                } // RRA | RRCA
                // TODO: It's probably a good idea to merge these 2 branches above ^
                0x37 => {
                    bus.registers.set_flag(Flag::Carry, Some(true))?;
                    bus.registers.set_flag(Flag::HalfCarry, Some(false))?;
                    bus.registers.set_flag(Flag::Subtract, Some(false))?;
                    Ok("scf".to_string())
                } // SCF
                0x3F => {
                    bus.registers.set_flag(Flag::Carry, None)?;
                    bus.registers.set_flag(Flag::HalfCarry, Some(false))?;
                    bus.registers.set_flag(Flag::Subtract, Some(false))?;
                    Ok("ccf".to_string())
                } // CCF
                0x2F => {
                    bus.registers.a = !bus.registers.a;
                    bus.registers.set_flag(Flag::Subtract, Some(true))?;
                    bus.registers.set_flag(Flag::HalfCarry, Some(true))?;
                    Ok("cpl".to_string())
                } // CPL
                0x27 => arithmetic::daa(bus), // DAA
                0xCB => Self::prefixed_instr(bus),
                0xD3 | 0xDB | 0xDD | 0xE3 | 0xE4 | 0xEB..0xEE | 0xF4 | 0xFC | 0xFD => {
                    Err(GBError::IllegalInstruction(opcode))
                }
                _ => Ok("<unsupported>".to_string()),
            };
            Self::handle_result(result, opcode, opcode_addr)?;
            if bus.registers.ime && opcode != 0xFB {
                Self::handle_interupts(bus)?;
            }
        }
    }

    fn handle_interupts(bus: &mut Bus) -> Result<(), GBError> {
        for i in 0..5 {
            if alu::read_bits(bus.memory.io[0x0F], i, 1) == 1
                && alu::read_bits(bus.memory.ie, i, 1) == 1
            {
                if !bus.registers.ime {
                    bus.registers.exec = true;
                    return Ok(());
                }
                bus.registers.ime = false;
                bus.registers.exec = true;
                bus.memory.io[0x0F] = alu::set_bit(bus.memory.io[0x0F], i, false);
                bus.tick();
                bus.tick();
                let target_address = (0x40 + 8 * i) as u16;
                bus.registers.sp -= 1;
                bus.write(bus.registers.sp, (bus.registers.pc >> 8) as u8)?;
                bus.registers.sp -= 1;
                bus.write(bus.registers.sp, (bus.registers.pc & 0xFF) as u8)?;
                bus.registers.pc = target_address;
                bus.tick();
                break;
            }
        }
        Ok(())
    }

    fn handle_result(
        result: Result<String, GBError>,
        opcode: u8,
        opcode_addr: u16,
    ) -> Result<(), GBError> {
        match result {
            Ok(s) => {
                debug!("{:#X}: {:#X} -> {}", opcode_addr, opcode, s);
                // trace!("{:?}", registers);
                Ok(())
            }
            Err(err) => match err {
                GBError::IllegalAddress(_) | GBError::ReadOnlyAddress(_) => Ok(()),
                _ => Err(err),
            },
        }
    }
    fn prefixed_instr(bus: &mut Bus) -> Result<String, GBError> {
        let opcode = bus.fetch();
        match opcode {
            0x0..0x20 => bitwise::rotate_to_carry(opcode, bus), // RL R8 | RR R8 | RLC R8 | RRC R8
            0x20..0x30 | 0x38..0x40 => bitwise::shift_to_carry(opcode, bus), // SRL R8 | SLA R8 | SRA R8
            0x30..0x38 => bitwise::swap(opcode, bus),                        // SWAP R8
            0x40..0x80 => bitwise::test_bit(opcode, bus),                    // BIT U3 R8
            0x80..0xC0 => bitwise::reset_bit(opcode, bus),                   // RES U3 R8
            0xC0..=0xFF => bitwise::set_bit(opcode, bus),                    // SET U3 R8
        }
    }
}
