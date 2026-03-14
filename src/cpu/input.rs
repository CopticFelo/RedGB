use sdl3::keyboard::Keycode;

use crate::cpu::{alu, cpu_context::CpuContext};

#[derive(Default)]
pub struct Joypad {
    a: bool,
    b: bool,
    select: bool,
    start: bool,
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

impl Joypad {
    pub fn update(&mut self, keycode: Keycode, is_down: bool) {
        match keycode {
            Keycode::Z => self.a = is_down,
            Keycode::X => self.b = is_down,
            Keycode::C => self.select = is_down,
            Keycode::Return => self.start = is_down,
            Keycode::Up => self.up = is_down,
            Keycode::Down => self.down = is_down,
            Keycode::Left => self.left = is_down,
            Keycode::Right => self.right = is_down,
            _ => (),
        }
    }
    pub fn query_joypad(context: &mut CpuContext) {
        if alu::read_bits(context.memory.io[0], 5, 1) == 0 {
            context.memory.io[0] = alu::set_bit(context.memory.io[0], 0, !context.joypad.a);
            context.memory.io[0] = alu::set_bit(context.memory.io[0], 1, !context.joypad.b);
            context.memory.io[0] = alu::set_bit(context.memory.io[0], 2, !context.joypad.select);
            context.memory.io[0] = alu::set_bit(context.memory.io[0], 3, !context.joypad.start);
            context.memory.io[0x0F] = alu::set_bit(context.memory.io[0x0F], 4, true);
        }
        if alu::read_bits(context.memory.io[0], 4, 1) == 0 {
            context.memory.io[0] = alu::set_bit(context.memory.io[0], 0, !context.joypad.right);
            context.memory.io[0] = alu::set_bit(context.memory.io[0], 1, !context.joypad.left);
            context.memory.io[0] = alu::set_bit(context.memory.io[0], 2, !context.joypad.up);
            context.memory.io[0] = alu::set_bit(context.memory.io[0], 3, !context.joypad.down);
            context.memory.io[0x0F] = alu::set_bit(context.memory.io[0x0F], 4, true);
        }
    }
}
