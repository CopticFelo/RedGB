use std::collections::VecDeque;

use crate::{
    cpu::alu,
    error::GBError,
    mem::map::Memory,
    ppu::{ppu::DrawLayer, sprite::GBSprite},
};

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
    pub obj_priority: Option<u8>,
}

pub struct Fetcher {
    current_tile_id: u8,
    tile_x_step: u8,
    tile_y_step: u8,
    pub phase: u8,
    pub lx: u8,
    tile_hi: u8,
    tile_lo: u8,
    pub current_sprite: Option<GBSprite>,
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
            current_sprite: None,
        }
    }
}

impl Fetcher {
    pub fn fetch_bg_tile(&mut self, mem: &Memory, draw_layer: &DrawLayer) -> Result<u8, GBError> {
        let scy = mem.io[SCY];
        let scx = mem.io[SCX];
        let wy = mem.io[WY];
        let wx = mem.io[WX] as isize - 7;
        let ly = mem.io[LY];
        let lcdc = mem.io[LCDC];
        let is_window = *draw_layer == DrawLayer::Window;
        let tile_addr: u16 = {
            if is_window && (self.lx as isize - wx) >= 0 {
                if ly < wy {
                    return Ok(0);
                }
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
        self.phase = (self.phase + 1) & 3;
        Ok(2)
    }
    pub fn fetch_tile_data(&mut self, mem: &Memory, draw_layer: &DrawLayer) -> Result<u8, GBError> {
        let lcdc = mem.io[LCDC];
        let ly = mem.io[LY] as usize;
        if let Some(sprite) = self.current_sprite {
            let tile_height = if alu::read_bits(lcdc, 2, 1) == 1 {
                15
            } else {
                7
            };
            let mut tile_row = (ly as isize - sprite.y as isize) as u8;
            if sprite.y_flip {
                tile_row = tile_height - tile_row;
            }
            let tile_id = if tile_row < 7 {
                sprite.tile_index
            } else {
                sprite.tile_index.saturating_add(0)
            };
            let addr = 0x8000 + (tile_id as usize * 16) + (2 * tile_row as usize);
            if self.phase == 1 {
                self.tile_lo = mem.dma_read(addr).unwrap();
            } else {
                self.tile_hi = mem.dma_read(addr + 1).unwrap();
            }
            self.phase = (self.phase + 1) & 3;
            return Ok(0);
        }
        let scy = mem.io[SCY] as usize;
        let wy = mem.io[WY];
        let base_ptr = if alu::read_bits(lcdc, 4, 1) == 1 {
            0x8000
        } else {
            0x9000
        };
        let is_window = *draw_layer == DrawLayer::Window;
        let tile_row = if is_window {
            if ly < wy as usize {
                return Ok(0);
            }
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
        if (!fifo.is_empty() && self.current_sprite.is_none()) || self.phase != 3 {
            return;
        }
        for i in (0..8).rev() {
            let color_id = {
                if let Some(sprite) = self.current_sprite
                    && sprite.x_flip
                {
                    let lo = alu::read_bits(self.tile_lo, 7 - i, 1);
                    let hi = alu::read_bits(self.tile_hi, 7 - i, 1) << 1;
                    hi | lo
                } else {
                    let lo = alu::read_bits(self.tile_lo, i, 1);
                    let hi = alu::read_bits(self.tile_hi, i, 1) << 1;
                    hi | lo
                }
            };

            if let Some(sprite) = self.current_sprite {
                if fifo
                    .get(7 - i as usize)
                    .is_some_and(|pixel| pixel.color_id == 0)
                {
                    continue;
                }
                match fifo.get_mut(7 - i as usize) {
                    Some(pixel) => {
                        *pixel = Pixel {
                            color_id,
                            palette: mem.io[if sprite.dmg_palette == 0 { OBP0 } else { OBP1 }],
                            cgb_priority: None,
                            bg_priority: None,
                            obj_priority: Some(sprite.priority),
                        }
                    }
                    None => fifo.push_back(Pixel {
                        color_id,
                        palette: mem.io[if sprite.dmg_palette == 0 { OBP0 } else { OBP1 }],
                        cgb_priority: None,
                        bg_priority: None,
                        obj_priority: Some(sprite.priority),
                    }),
                }
            } else {
                fifo.push_back(Pixel {
                    color_id,
                    palette: mem.io[BGP],
                    cgb_priority: None,
                    bg_priority: None,
                    obj_priority: None,
                });
                self.lx = self.lx.saturating_add(1);
            }
        }
        self.phase = (self.phase + 1) & 3;
        self.current_sprite = None;
    }
    pub fn switch_to_sprite(&mut self, sprite: GBSprite) {
        self.current_sprite = Some(sprite);
        self.phase = 0;
    }
}
