use std::collections::VecDeque;

use crate::{cpu::alu, error::GBError, mem::map::Memory};

const LCDC: usize = 0x40;
const LY: usize = 0x44;
const BGP: usize = 0x47;
const OBP0: usize = 0x48;
const OBP1: usize = 0x49;
const SCY: usize = 0x42;
const SCX: usize = 0x43;
const WY: usize = 0x4A;
const WX: usize = 0x4B;

pub struct Pixel {
    pub color_id: u8,
    pub palette: u8,
    pub cgb_priority: Option<u8>,
    pub bg_priority: Option<u8>,
}

pub struct Fetcher {
    current_tile_id: u8,
    tile_x_step: u8,
    tile_y_step: u8,
    pub phase: u8,
    pub lx: u8,
    tile_hi: u8,
    tile_lo: u8,
}

impl Default for Fetcher {
    fn default() -> Self {
        Self {
            current_tile_id: 0,
            tile_x_step: 0,
            tile_y_step: 0,
            phase: 0,
            tile_hi: 0,
            tile_lo: 0,
            lx: 0,
        }
    }
}

impl Fetcher {
    pub fn fetch_bg_tile(&mut self, mem: &Memory) -> Result<u8, GBError> {
        let scy = mem.io[SCY];
        let scx = mem.io[SCX];
        let wy = mem.io[WY];
        let wx = mem.io[WX] as isize - 7;
        let ly = mem.io[LY];
        let lcdc = mem.io[LCDC];
        let is_window = alu::read_bits(lcdc, 5, 1) == 1
            && (wx..(wx + 160)).contains(&(self.lx as isize))
            && wy <= ly;
        let tile_addr: u16 = {
            if is_window && (self.lx as isize - wx) >= 0 {
                let base = 0x9800;
                let tile_map = (alu::read_bits(lcdc, 6, 1) as u16) << 10;
                let tile_map_y = ((ly - wy) as u16 >> 3) << 5;
                let tile_map_x = (self.lx as isize - wx) as u16 >> 3;
                base | tile_map | tile_map_y | tile_map_x
            } else {
                let base = 0x9800;
                let tile_map = (alu::read_bits(lcdc, 3, 1) as u16) << 10;
                let tile_map_y = (((ly as u16 + scy as u16) / 8) & 31) << 5;
                let tile_map_x = ((self.lx as u16 + scx as u16) / 8) & 31;
                base | tile_map | tile_map_y | tile_map_x
            }
        };
        self.current_tile_id = mem.dma_read(tile_addr as usize)?;
        if self.current_tile_id != 0 {
            print!("");
        }
        self.phase = (self.phase + 1) & 3;
        Ok(2)
    }
    pub fn fetch_tile_data(&mut self, mem: &Memory) -> Result<u8, GBError> {
        let scy = mem.io[SCY] as usize;
        let ly = mem.io[LY] as usize;
        let lcdc = mem.io[LCDC];
        let wy = mem.io[WY];
        let wx = mem.io[WX] as isize - 7;
        let base_ptr = if alu::read_bits(lcdc, 4, 1) == 1 {
            0x8000
        } else {
            0x9000
        };
        let is_window = alu::read_bits(lcdc, 5, 1) == 1
            && (wx..wx + 160).contains(&(self.lx as isize))
            && wy as usize <= ly;
        let tile_row = if is_window {
            (ly - wy as usize) & 7
        } else {
            (ly + scy) & 7
        };
        let addr = if base_ptr == 0x8000 {
            base_ptr + (16_usize * self.current_tile_id as usize)
        } else {
            (base_ptr as isize + (16_isize * self.current_tile_id as i8 as isize)) as usize
        } + (2 * tile_row);
        if self.phase == 1 {
            self.tile_lo = mem.dma_read(addr).unwrap();
        } else {
            self.tile_hi = mem.dma_read(addr + 1).unwrap();
        }
        self.phase = (self.phase + 1) & 3;
        Ok(2)
    }
    pub fn push_to_fifo(&mut self, mem: &Memory, fifo: &mut VecDeque<Pixel>) {
        if !fifo.is_empty() || self.phase != 3 {
            return;
        }
        for i in (0..8).rev() {
            let color_id = {
                let lo = alu::read_bits(self.tile_lo, i, 1);
                let hi = alu::read_bits(self.tile_hi, i, 1) << 1;
                hi | lo
            };
            fifo.push_back(Pixel {
                color_id,
                palette: mem.io[BGP],
                cgb_priority: None,
                bg_priority: None,
            });
            self.lx = self.lx.saturating_add(1);
        }
        self.phase = (self.phase + 1) & 3;
    }
}
