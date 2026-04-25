extern crate sdl3;

use std::cmp::{self, Ordering};
use std::collections::VecDeque;

use log::trace;

use crate::bus::Bus;
use crate::cpu::alu;
use crate::error::GBError;
use crate::mem::map::Memory;
use crate::ppu::fetcher::{Fetcher, Pixel};
use crate::ppu::ppumode::*;
use crate::ppu::sprite::GBSprite;

const IF: usize = 0x0F;
const LCDC: usize = 0x40;
const STAT: usize = 0x41;
const LY: usize = 0x44;
const LYC: usize = 0x45;
const SCX: usize = 0x43;
const WY: usize = 0x4A;
const WX: usize = 0x4B;

pub struct PPU {
    last_cycle: u64,
    mode: PPUMode,
    lx: u8,
    pub framebuffer: Vec<u8>,
    pub frame_flag: bool,
    current_oam: VecDeque<GBSprite>,
    bg_fifo: VecDeque<Pixel>,
    oam_fifo: VecDeque<Pixel>,
    fetcher: Fetcher,
    discard_counter: u8,
    cycle_deficit: u64,
}

impl Default for PPU {
    fn default() -> Self {
        Self {
            frame_flag: false,
            mode: PPUMode::Scan,
            last_cycle: 0,
            lx: 0,
            framebuffer: vec![0x0; 160 * 144 * 3],
            current_oam: VecDeque::with_capacity(10),
            bg_fifo: VecDeque::with_capacity(8),
            oam_fifo: VecDeque::with_capacity(8),
            fetcher: Fetcher::default(),
            discard_counter: 0,
            cycle_deficit: 0,
        }
    }
}

impl PPU {
    #[inline]
    fn inc_ly(mem: &mut Memory) {
        mem.io[LY] = (mem.io[LY] + 1) % 154;
        let lyc_interrupt = alu::read_bits(mem.io[STAT], 6, 1) == 1;
        if mem.io[LY] == mem.io[LYC] && lyc_interrupt {
            mem.io[IF] = alu::set_bit(mem.io[IF], 1, true);
            mem.io[STAT] = alu::set_bit(mem.io[STAT], 2, true);
        } else {
            mem.io[STAT] = alu::set_bit(mem.io[STAT], 2, false);
        }
    }
    fn colorise(bg_pixel: &Pixel, obj_pixel: &Option<Pixel>) -> [u8; 3] {
        let visible_pixel = if let Some(sprite_pixel) = obj_pixel
            && sprite_pixel.color_id != 0
            && ((sprite_pixel.obj_priority == Some(1) && bg_pixel.color_id == 0)
                || sprite_pixel.obj_priority == Some(0))
        {
            sprite_pixel
        } else {
            bg_pixel
        };
        let palette = visible_pixel.palette;
        let ids = [
            alu::read_bits(palette, 0, 2),
            alu::read_bits(palette, 2, 2),
            alu::read_bits(palette, 4, 2),
            alu::read_bits(palette, 6, 2),
        ];
        match ids[visible_pixel.color_id as usize] {
            0 => [0xFF, 0xFF, 0xFF],
            1 => [0xD3, 0xD3, 0xD3],
            2 => [0x69, 0x69, 0x69],
            3 => [0x0, 0x0, 0x0],
            _ => unreachable!(),
        }
    }

    pub fn tick(&mut self, mem: &mut Memory, t_cycles: &u64) {
        let delta = t_cycles.abs_diff(self.last_cycle);
        if self.cycle_deficit > 0 {
            self.cycle_deficit -= 4;
            return;
        }
        match self.mode {
            PPUMode::Scan => {
                self.mode.stat_interrupt(mem);
                self.last_cycle = *t_cycles;
                self.current_oam.extend(PPU::fetch_from_oam(mem).unwrap());
                self.discard_counter = mem.io[SCX] & 7;
                self.mode = PPUMode::Draw(DrawLayer::Bg);
                // NOTE: You need another 4 cycles here (so 80+4) cuz the gb wastes 4 cycles
                // fetching the first tile twice or something
                self.cycle_deficit += 84;
            }
            PPUMode::Draw(draw_layer) => {
                for _ in 0..2 {
                    let _ = match self.fetcher.phase {
                        0 => self.fetcher.fetch_bg_tile(mem, &draw_layer),
                        1 | 2 => self.fetcher.fetch_tile_data(mem, &draw_layer),
                        _ => Ok(0),
                    };
                }
                if self.fetcher.current_sprite.is_some() {
                    self.fetcher.push_to_fifo(mem, &mut self.oam_fifo, self.lx);
                } else {
                    self.fetcher.push_to_fifo(mem, &mut self.bg_fifo, self.lx);
                }
                if !self.bg_fifo.is_empty() {
                    self.fifo_pop(mem);
                }
                if self.lx > 159 {
                    self.mode = PPUMode::HBlank;
                    self.bg_fifo.clear();
                    self.oam_fifo.clear();
                    self.current_oam.clear();
                    self.fetcher.phase = 0;
                    self.lx = 0;
                    self.fetcher.lx = 0;
                }
            }
            PPUMode::HBlank => {
                if alu::read_bits(mem.io[LCDC], 5, 1) == 1
                    && mem.io[LY] >= mem.io[WY]
                    && (0..167).contains(&mem.io[WX])
                {
                    self.fetcher.window_ly += 1;
                }
                self.mode.stat_interrupt(mem);
                let draw_len = delta - 80;
                self.cycle_deficit += 456 - draw_len;
                PPU::inc_ly(mem);
                if mem.io[LY] == 144 {
                    self.mode = PPUMode::VBlank;
                } else {
                    self.mode = PPUMode::Scan;
                }
                self.bg_fifo.clear();
                trace!("LY: {}", mem.io[LY]);
                trace!("Framebuffer: {:#?}", &self.framebuffer[0..20]);
            }
            PPUMode::VBlank => {
                self.fetcher.window_ly = 0;
                if mem.io[LY] == 144 {
                    self.frame_flag = true;
                    self.mode.stat_interrupt(mem);
                    mem.io[IF] = alu::set_bit(mem.io[IF], 0, true);
                } else if mem.io[LY] == 153 {
                    self.mode = PPUMode::Scan;
                }
                PPU::inc_ly(mem);
                self.cycle_deficit += 456;
            }
        }
    }
    fn determine_layer(&mut self, mem: &Memory) -> (DrawLayer, bool) {
        if self
            .current_oam
            .front()
            .is_some_and(|obj| (obj.x..obj.x + 8).contains(&(self.lx as i16)))
        {
            let obj = self.current_oam.pop_front().unwrap();
            return (
                DrawLayer::Obj(obj),
                self.mode != PPUMode::Draw(DrawLayer::Obj(obj)),
            );
        }
        let wy = mem.io[WY];
        let wx = mem.io[WX] as isize - 7;
        let lcdc = mem.io[LCDC];
        let is_window = alu::read_bits(lcdc, 5, 1) == 1
            && (wx..(wx + 160)).contains(&(self.lx as isize))
            && wy <= mem.io[LY];
        if is_window {
            (DrawLayer::Window, self.mode == PPUMode::Draw(DrawLayer::Bg))
        } else {
            (DrawLayer::Bg, self.mode != PPUMode::Draw(DrawLayer::Bg))
        }
    }
    fn fifo_pop(&mut self, mem: &Memory) {
        if self.fetcher.current_sprite.is_some() {
            return;
        }
        for _ in 0..4 {
            if self.bg_fifo.is_empty() {
                return;
            }
            let layer_query = self.determine_layer(mem);
            match layer_query {
                (DrawLayer::Window, true) => {
                    self.mode = PPUMode::Draw(DrawLayer::Window);
                    self.bg_fifo.clear();
                    self.fetcher.lx = self.lx;
                    self.fetcher.phase = 0;
                    break;
                }
                (DrawLayer::Window, false) => {
                    self.mode = PPUMode::Draw(DrawLayer::Window);
                }
                (DrawLayer::Bg, true) => {
                    self.mode = PPUMode::Draw(DrawLayer::Bg);
                }
                (DrawLayer::Obj(sprite), true) => {
                    self.fetcher.switch_to_sprite(sprite);
                    self.mode = PPUMode::Draw(layer_query.0);
                    return;
                }
                _ => (),
            }
            let bg_pixel = self.bg_fifo.pop_front().unwrap();
            let obj_pixel = self.oam_fifo.pop_front();
            if self.mode == PPUMode::Draw(DrawLayer::Window) {
                self.discard_counter = 0;
            }
            if self.discard_counter == 0 {
                let framebuffer_index = ((mem.io[LY] as usize * 160) + self.lx as usize) * 3;
                if alu::read_bits(mem.io[LCDC], 0, 1) == 0
                    && self.mode == PPUMode::Draw(DrawLayer::Bg)
                {
                    self.framebuffer[framebuffer_index..framebuffer_index + 3]
                        .copy_from_slice(&[0xFF, 0xFF, 0xFF]);
                } else {
                    self.framebuffer[framebuffer_index..framebuffer_index + 3]
                        .copy_from_slice(&PPU::colorise(&bg_pixel, &obj_pixel));
                }
                self.lx += 1;
                if self.lx == 160 {
                    break;
                }
            } else {
                self.discard_counter -= 1;
            }
        }
    }
    fn fetch_from_oam(mem: &Memory) -> Result<Vec<GBSprite>, GBError> {
        let mut sprite_table: Vec<GBSprite> = Vec::new();
        let ly = mem.io[LY];
        let mut index = 0;
        for obj_addr in (0xFE00..0xFEA0).step_by(4) {
            let y = (mem.dma_read(obj_addr)? as u16 as i16) - 16;
            let x = (mem.dma_read(obj_addr + 1)? as u16 as i16) - 8;
            let mut tile_index = mem.dma_read(obj_addr + 2)?;
            let attributes = mem.dma_read(obj_addr + 3)?;
            let obj_size = if alu::read_bits(mem.io[LCDC], 2, 1) == 1 {
                tile_index &= 0xFE;
                15
            } else {
                7
            };
            if ((ly as isize - obj_size)..=ly as isize).contains(&(y as isize))
                && index < 10
                && (-7..168).contains(&x)
            {
                sprite_table.push(GBSprite {
                    x,
                    y,
                    tile_index,
                    priority: alu::read_bits(attributes, 7, 1),
                    y_flip: alu::read_bits(attributes, 6, 1) == 1,
                    x_flip: alu::read_bits(attributes, 5, 1) == 1,
                    dmg_palette: alu::read_bits(attributes, 4, 1),
                    bank: alu::read_bits(attributes, 3, 1),
                    cgb_palette: alu::read_bits(attributes, 0, 3),
                });
                index += 1;
            }
        }
        sprite_table.sort_by(|sprite_a, sprite_b| sprite_a.x.cmp(&sprite_b.x));
        Ok(sprite_table)
    }
}
