use redgb::{
    cpu::{
        alu,
        clock::Clock,
        cpu_context::CpuContext,
        reg_file::{Flag, Modes, RegFile},
    },
    mem::map::MemoryMap,
    rom::rom_info::ROMInfo,
};

fn get_mock_context(rom: Vec<u8>) -> CpuContext {
    let mut context = CpuContext::init(
        RegFile::new(Modes::CGBDMG),
        MemoryMap::init_rom(rom, ROMInfo::default()),
        Clock::default(),
    );
    context.registers.pc = 0;
    context
}

#[test]
fn add_a_b() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x80, 0xDD]);
    context.registers.a = 172;
    context.registers.b = 108;
    let _ = context.start_exec_cycle();
    assert_eq!(context.registers.a, 24);
    assert_eq!(context.clock.m_cycles, 2);
    assert!(context.registers.read_flag(Flag::Carry));
    assert!(context.registers.read_flag(Flag::HalfCarry));
    assert!(!context.registers.read_flag(Flag::Zero));
    assert!(!context.registers.read_flag(Flag::Subtract));

    context.registers.a = 0;
    context.registers.b = 0;
    context.registers.pc = 0;
    let _ = context.start_exec_cycle();
    assert_eq!(context.clock.m_cycles, 4);
    assert!(!context.registers.read_flag(Flag::Carry));
    assert!(!context.registers.read_flag(Flag::HalfCarry));
    assert!(context.registers.read_flag(Flag::Zero));
    assert!(!context.registers.read_flag(Flag::Subtract));
    Ok(())
}

#[test]
fn adc_a_hl() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x8E, 0xDD]);
    context.registers.a = 172;
    alu::write_u16(&mut context.registers.l, &mut context.registers.h, 0xC001);
    let _ = context.memory.write(&mut context.clock, 0xC001, 108);
    let _ = context.registers.set_flag(Flag::Carry, Some(true));
    let _ = context.start_exec_cycle();
    assert_eq!(context.registers.a, 25);
    assert_eq!(context.clock.m_cycles, 4);
    assert!(context.registers.read_flag(Flag::Carry));
    assert!(context.registers.read_flag(Flag::HalfCarry));
    assert!(!context.registers.read_flag(Flag::Zero));
    assert!(!context.registers.read_flag(Flag::Subtract));

    context.registers.a = 255;
    let _ = context.memory.write(&mut context.clock, 0xC001, 0);
    context.registers.pc = 0;
    let _ = context.start_exec_cycle();
    assert_eq!(context.clock.m_cycles, 8);
    assert!(context.registers.read_flag(Flag::Carry));
    assert!(context.registers.read_flag(Flag::HalfCarry));
    assert!(context.registers.read_flag(Flag::Zero));
    assert!(!context.registers.read_flag(Flag::Subtract));
    Ok(())
}

#[test]
fn sub_a_c() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x91, 0xDD]);
    context.registers.a = 64;
    context.registers.c = 108;
    let _ = context.start_exec_cycle();
    assert_eq!(context.registers.a, 212);
    assert_eq!(context.clock.m_cycles, 2);
    assert!(context.registers.read_flag(Flag::Carry));
    assert!(context.registers.read_flag(Flag::HalfCarry));
    assert!(!context.registers.read_flag(Flag::Zero));
    assert!(context.registers.read_flag(Flag::Subtract));

    context.registers.a = 0;
    context.registers.c = 0;
    context.registers.pc = 0;
    let _ = context.start_exec_cycle();
    assert_eq!(context.clock.m_cycles, 4);
    assert!(!context.registers.read_flag(Flag::Carry));
    assert!(!context.registers.read_flag(Flag::HalfCarry));
    assert!(context.registers.read_flag(Flag::Zero));
    assert!(context.registers.read_flag(Flag::Subtract));
    Ok(())
}

#[test]
fn sbc_a_hl() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x9E, 0xDD]);
    context.registers.a = 64;
    alu::write_u16(&mut context.registers.l, &mut context.registers.h, 0xC001);
    let _ = context.memory.write(&mut context.clock, 0xC001, 108);
    let _ = context.registers.set_flag(Flag::Carry, Some(true));
    let _ = context.start_exec_cycle();
    assert_eq!(context.registers.a, 211);
    assert_eq!(context.clock.m_cycles, 4);
    assert!(context.registers.read_flag(Flag::Carry));
    assert!(context.registers.read_flag(Flag::HalfCarry));
    assert!(!context.registers.read_flag(Flag::Zero));
    assert!(context.registers.read_flag(Flag::Subtract));

    context.registers.a = 1;
    let _ = context.memory.write(&mut context.clock, 0xC001, 0);
    context.registers.pc = 0;
    let _ = context.start_exec_cycle();
    assert_eq!(context.clock.m_cycles, 8);
    assert!(!context.registers.read_flag(Flag::Carry));
    assert!(!context.registers.read_flag(Flag::HalfCarry));
    assert!(context.registers.read_flag(Flag::Zero));
    assert!(context.registers.read_flag(Flag::Subtract));
    Ok(())
}

#[test]
fn inc_b() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x04, 0xDD]);
    context.registers.b = 255;
    let _ = context.start_exec_cycle();
    assert_eq!(context.registers.b, 0);
    assert_eq!(context.clock.m_cycles, 2);
    assert!(context.registers.read_flag(Flag::HalfCarry));
    assert!(context.registers.read_flag(Flag::Zero));
    assert!(!context.registers.read_flag(Flag::Subtract));
    Ok(())
}

#[test]
fn dec_hl() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x35, 0xDD]);
    alu::write_u16(&mut context.registers.l, &mut context.registers.h, 0xC001);
    let _ = context.memory.write(&mut context.clock, 0xC001, 0);
    let _ = context.start_exec_cycle();
    assert_eq!(
        context.memory.read(&mut context.clock, 0xC001).unwrap(),
        255
    );
    assert_eq!(context.clock.m_cycles, 6);
    assert!(context.registers.read_flag(Flag::HalfCarry));
    assert!(!context.registers.read_flag(Flag::Zero));
    assert!(context.registers.read_flag(Flag::Subtract));
    Ok(())
}
