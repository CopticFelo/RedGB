use crate::{cpu::alu, mem::map::Memory, ppu::sprite::GBSprite};

const STAT: usize = 0x41;
const IF: usize = 0x0F;

#[derive(PartialEq, Clone, Copy)]
pub enum DrawLayer {
    Bg,
    Obj(GBSprite),
    Window,
}

#[derive(PartialEq, Clone, Copy)]
pub enum PPUMode {
    HBlank,
    VBlank,
    Scan,
    Draw(DrawLayer),
}

impl PPUMode {
    /// Should be called ONLY at the BEGINNING of each mode
    pub fn stat_interrupt(&mut self, mem: &mut Memory) {
        match *self {
            // Mode 2
            PPUMode::Scan => {
                mem.io[STAT] = alu::set_bit(mem.io[STAT], 0, false);
                mem.io[STAT] = alu::set_bit(mem.io[STAT], 1, true);
                let is_interrupt = alu::read_bits(mem.io[STAT], 5, 1) == 1;
                if is_interrupt {
                    mem.io[IF] = alu::set_bit(mem.io[IF], 1, true);
                }
            }
            // Mode 3
            PPUMode::Draw(_) => {
                mem.io[STAT] = alu::set_bit(mem.io[STAT], 0, true);
                mem.io[STAT] = alu::set_bit(mem.io[STAT], 1, true);
            }
            // Mode 0
            PPUMode::HBlank => {
                mem.io[STAT] = alu::set_bit(mem.io[STAT], 0, false);
                mem.io[STAT] = alu::set_bit(mem.io[STAT], 1, false);
                let is_interrupt = alu::read_bits(mem.io[STAT], 3, 1) == 1;
                if is_interrupt {
                    mem.io[IF] = alu::set_bit(mem.io[IF], 1, true);
                }
            }
            // Mode 1
            PPUMode::VBlank => {
                mem.io[STAT] = alu::set_bit(mem.io[STAT], 0, true);
                mem.io[STAT] = alu::set_bit(mem.io[STAT], 1, false);
                let is_interrupt = alu::read_bits(mem.io[STAT], 4, 1) == 1;
                if is_interrupt {
                    mem.io[IF] = alu::set_bit(mem.io[IF], 1, true);
                }
            }
        }
    }
}
