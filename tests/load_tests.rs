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
fn ld_b_l() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x45, 0xDD]);
    let _ = context.start_exec_cycle();
    assert_eq!(context.registers.l, context.registers.b);
    assert_eq!(context.clock.m_cycles, 2);
    Ok(())
}

#[test]
fn ld_b_hl() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x46, 0xDD]);
    let _ = context.memory.write(&mut context.clock, 0xC001, 0xB1);
    alu::write_u16(&mut context.registers.l, &mut context.registers.h, 0xC001);
    let _ = context.start_exec_cycle();
    assert_eq!(
        context
            .memory
            .read(
                &mut context.clock,
                alu::read_u16(&context.registers.l, &context.registers.h)
            )
            .unwrap(),
        context.registers.b
    );
    assert_eq!(context.clock.m_cycles, 5);
    Ok(())
}

#[test]
fn ld_hl_a() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x77, 0xDD]);
    context.registers.a = 0xB1;
    alu::write_u16(&mut context.registers.l, &mut context.registers.h, 0xC001);
    let _ = context.start_exec_cycle();
    assert_eq!(
        context
            .memory
            .read(
                &mut context.clock,
                alu::read_u16(&context.registers.l, &context.registers.h)
            )
            .unwrap(),
        context.registers.a
    );
    assert_eq!(context.clock.m_cycles, 4);
    Ok(())
}

#[test]
fn ld_hl_n8() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x36, 0x67, 0xDD]);
    alu::write_u16(&mut context.registers.l, &mut context.registers.h, 0xC001);
    let _ = context.start_exec_cycle();
    assert_eq!(
        context
            .memory
            .read(
                &mut context.clock,
                alu::read_u16(&context.registers.l, &context.registers.h)
            )
            .unwrap(),
        0x67
    );
    assert_eq!(context.clock.m_cycles, 5);
    Ok(())
}

#[test]
fn ld_e_n8() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x1E, 0x67, 0xDD]);
    let _ = context.start_exec_cycle();
    assert_eq!(context.registers.e, 0x67);
    assert_eq!(context.clock.m_cycles, 3);
    Ok(())
}

#[test]
fn ld_de_n16() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x11, 0x34, 0x12, 0xDD]);
    let _ = context.start_exec_cycle();
    assert_eq!(
        alu::read_u16(&context.registers.e, &context.registers.d),
        0x1234
    );
    assert_eq!(context.clock.m_cycles, 4);
    Ok(())
}

#[test]
fn ld_sp_n16() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x31, 0x34, 0x12, 0xDD]);
    let _ = context.start_exec_cycle();
    assert_eq!(context.registers.sp, 0x1234);
    assert_eq!(context.clock.m_cycles, 4);
    Ok(())
}

#[test]
fn ld_bcmem_a() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x02, 0xDD]);
    context.registers.a = 10;
    alu::write_u16(&mut context.registers.c, &mut context.registers.b, 0xC001);
    let _ = context.start_exec_cycle();
    assert_eq!(context.memory.read(&mut context.clock, 0xC001).unwrap(), 10);
    assert_eq!(context.clock.m_cycles, 4);
    Ok(())
}

#[test]
fn ld_hlimem_a() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x22, 0xDD]);
    context.registers.a = 10;
    alu::write_u16(&mut context.registers.l, &mut context.registers.h, 0xC001);
    let _ = context.start_exec_cycle();
    assert_eq!(context.memory.read(&mut context.clock, 0xC001).unwrap(), 10);
    assert_eq!(
        alu::read_u16(&context.registers.l, &context.registers.h),
        0xC002
    );
    assert_eq!(context.clock.m_cycles, 4);
    Ok(())
}

#[test]
fn ld_hldmem_a() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x32, 0xDD]);
    context.registers.a = 10;
    alu::write_u16(&mut context.registers.l, &mut context.registers.h, 0xC001);
    let _ = context.start_exec_cycle();
    assert_eq!(context.memory.read(&mut context.clock, 0xC001).unwrap(), 10);
    assert_eq!(
        alu::read_u16(&context.registers.l, &context.registers.h),
        0xC001 - 1
    );
    assert_eq!(context.clock.m_cycles, 4);
    Ok(())
}

#[test]
fn ld_a_demem() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x1A, 0xDD]);
    alu::write_u16(&mut context.registers.e, &mut context.registers.d, 0xC001);
    context
        .memory
        .write(&mut context.clock, 0xC001, 18)
        .unwrap();
    let _ = context.start_exec_cycle();
    assert_eq!(context.registers.a, 18);
    assert_eq!(context.clock.m_cycles, 4);
    Ok(())
}

#[test]
fn ld_a_hlimem() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x2A, 0xDD]);
    alu::write_u16(&mut context.registers.l, &mut context.registers.h, 0xC001);
    context
        .memory
        .write(&mut context.clock, 0xC001, 18)
        .unwrap();
    let _ = context.start_exec_cycle();
    assert_eq!(context.registers.a, 18);
    assert_eq!(
        alu::read_u16(&context.registers.l, &context.registers.h),
        0xC001 + 1
    );
    assert_eq!(context.clock.m_cycles, 4);
    Ok(())
}

#[test]
fn ld_a_hldmem() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x3A, 0xDD]);
    alu::write_u16(&mut context.registers.l, &mut context.registers.h, 0xC001);
    context
        .memory
        .write(&mut context.clock, 0xC001, 18)
        .unwrap();
    let _ = context.start_exec_cycle();
    assert_eq!(context.registers.a, 18);
    assert_eq!(
        alu::read_u16(&context.registers.l, &context.registers.h),
        0xC001 - 1
    );
    assert_eq!(context.clock.m_cycles, 4);
    Ok(())
}

#[test]
fn ld_n16mem_sp() -> Result<(), String> {
    let mut context = get_mock_context(vec![0x8, 0x1, 0xC0, 0xDD]);
    context.registers.sp = 0x6767;
    let _ = context.start_exec_cycle();
    assert_eq!(
        context.registers.sp,
        alu::read_u16(
            &context.memory.read(&mut context.clock, 0xC001).unwrap(),
            &context.memory.read(&mut context.clock, 0xC002).unwrap()
        )
    );
    assert_eq!(context.clock.m_cycles, 8);
    Ok(())
}

#[test]
fn ld_sp_hl() -> Result<(), String> {
    let mut context = get_mock_context(vec![0xF9, 0xDD]);
    alu::write_u16(&mut context.registers.l, &mut context.registers.h, 0x6767);
    let _ = context.start_exec_cycle();
    assert_eq!(
        context.registers.sp,
        alu::read_u16(&context.registers.l, &context.registers.h)
    );
    assert_eq!(context.clock.m_cycles, 3);
    Ok(())
}
