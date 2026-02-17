use crate::mem::map::MemoryMap;

#[derive(Debug, Default)]
pub struct Clock {
    pub t_cycles: u64,
}

// HACK: Incomplete understanding of how clocks work

impl Clock {
    pub fn tick(&mut self, ly: &mut u8) {
        self.t_cycles += 4_u64;

        // V-Blank
        // HACK: probably will change later
        if self.t_cycles.is_multiple_of(456) {
            *ly = ly.wrapping_add(1);
        }
    }
}
