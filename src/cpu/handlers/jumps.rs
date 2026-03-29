use crate::{bus::Bus, cpu::alu, error::GBError};

const CONDITION_NAMES: [&str; 4] = ["nz", "z", "nc", "c"];

pub fn jmp(bus: &mut Bus, opcode: u8, is_relative: bool) -> Result<String, GBError> {
    let target_address: u16;
    let is_conditional: bool;
    let mut log: String = String::new();
    if is_relative {
        log += "jr ";
        is_conditional = opcode != 0x18;
        let delta = bus.fetch() as i8;
        target_address = (bus.registers.pc as i16 + delta as i16) as u16;
    } else {
        log += "jp ";
        is_conditional = opcode != 0xC3;
        target_address = alu::read_u16(&bus.fetch(), &bus.fetch());
    }
    let condition = alu::read_bits(opcode, 3, 2);
    if is_conditional {
        log += CONDITION_NAMES[condition as usize];
    }
    log += &format!(" {:#X}", target_address);
    if bus.registers.match_condition(condition)? || !is_conditional {
        bus.registers.pc = target_address;
        bus.tick();
    }
    Ok(log)
}

pub fn call(bus: &mut Bus, opcode: u8) -> Result<String, GBError> {
    let target_address: u16 = alu::read_u16(&bus.fetch(), &bus.fetch());
    let is_conditional: bool = opcode != 0xCD;
    let mut log = String::from("call ");
    let condition = alu::read_bits(opcode, 3, 2);
    if is_conditional {
        log += CONDITION_NAMES[condition as usize];
    }
    log += &format!(" {:#X}", target_address);
    if bus.registers.match_condition(condition)? || !is_conditional {
        bus.registers.sp -= 1;
        bus.write(bus.registers.sp, (bus.registers.pc >> 8) as u8)?;
        bus.registers.sp -= 1;
        bus.write(bus.registers.sp, (bus.registers.pc & 0xFF) as u8)?;
        bus.registers.pc = target_address;
        bus.tick();
    }
    Ok(log)
}

pub fn ret(bus: &mut Bus, opcode: u8) -> Result<String, GBError> {
    let mut log = String::from("ret");
    if opcode == 0xD9 {
        bus.registers.ime = true;
        log += "i";
    }
    if opcode != 0xC9 && opcode != 0xD9 {
        let condition = alu::read_bits(opcode, 3, 2);
        log += &format!(" {}", CONDITION_NAMES[condition as usize]);
        bus.tick();
        if !bus
            .registers
            .match_condition(alu::read_bits(opcode, 3, 2))?
        {
            return Ok(log);
        }
    }
    let lsb = bus.read(bus.registers.sp)?;
    bus.registers.sp += 1;
    let msb = bus.read(bus.registers.sp)?;
    bus.registers.sp += 1;
    let addr = alu::read_u16(&lsb, &msb);
    bus.tick();
    bus.registers.pc = addr;
    Ok(log)
}

pub fn rst(bus: &mut Bus, opcode: u8) -> Result<String, GBError> {
    let addr = (alu::read_bits(opcode, 3, 3) * 8) as u16;
    bus.tick();
    bus.registers.sp -= 1;
    bus.write(bus.registers.sp, (bus.registers.pc >> 8) as u8)?;
    bus.registers.sp -= 1;
    bus.write(bus.registers.sp, (bus.registers.pc & 0xFF) as u8)?;
    bus.registers.pc = addr;
    Ok(format!("rst {:#X}", addr))
}
