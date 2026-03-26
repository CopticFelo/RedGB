use sdl3::keyboard::Keycode;

use crate::{cpu::alu, mem::map::MemoryMap};

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
    pub fn query_joypad(&mut self, mem: &mut MemoryMap) {
        if alu::read_bits(mem.io[0], 5, 1) == 0 {
            mem.io[0] = alu::set_bit(mem.io[0], 0, !self.a);
            mem.io[0] = alu::set_bit(mem.io[0], 1, !self.b);
            mem.io[0] = alu::set_bit(mem.io[0], 2, !self.select);
            mem.io[0] = alu::set_bit(mem.io[0], 3, !self.start);
            mem.io[0x0F] = alu::set_bit(mem.io[0x0F], 4, true);
        }
        if alu::read_bits(mem.io[0], 4, 1) == 0 {
            mem.io[0] = alu::set_bit(mem.io[0], 0, !self.right);
            mem.io[0] = alu::set_bit(mem.io[0], 1, !self.left);
            mem.io[0] = alu::set_bit(mem.io[0], 2, !self.up);
            mem.io[0] = alu::set_bit(mem.io[0], 3, !self.down);
            mem.io[0x0F] = alu::set_bit(mem.io[0x0F], 4, true);
        }
    }
}
