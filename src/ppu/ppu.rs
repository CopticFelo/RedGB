extern crate sdl3;

use std::collections::VecDeque;

use log::trace;

use crate::bus::Bus;
use crate::cpu::alu;
use crate::error::GBError;
use crate::mem::map::Memory;
use crate::ppu::fetcher::{Fetcher, Pixel};
use crate::ppu::sprite::GBSprite;

const IF: usize = 0x0F;
const LCDC: usize = 0x40;
const STAT: usize = 0x41;
const LY: usize = 0x44;
const LYC: usize = 0x45;
const BGP: usize = 0x47;
const OBP0: usize = 0x48;
const OBP1: usize = 0x49;
const SCY: usize = 0x42;
const SCX: usize = 0x43;
const WY: usize = 0x4A;
const WX: usize = 0x4B;

#[derive(PartialEq)]
enum DrawLayer {
    Bg,
    Obj,
    Window,
}

#[derive(PartialEq)]
enum PPUMode {
    HBlank,
    VBlank,
    Scan,
    Draw(DrawLayer),
}

impl PPUMode {
    fn stat_interrupt(&mut self, mem: &mut Memory) {
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

pub struct PPU {
    last_cycle: u64,
    mode: PPUMode,
    lx: u8,
    pub framebuffer: Vec<u8>,
    pub frame_flag: bool,
    current_oam: Option<[Option<GBSprite>; 10]>,
    bg_fifo: VecDeque<Pixel>,
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
            current_oam: None,
            bg_fifo: VecDeque::with_capacity(8),
            fetcher: Fetcher::default(),
            discard_counter: 0,
            cycle_deficit: 0,
        }
    }
}

impl PPU {
    #[inline(always)]
    fn check_lyc(mem: &mut Memory) {
        let lyc_interrupt = alu::read_bits(mem.io[STAT], 6, 1) == 1;
        if mem.io[LY] == mem.io[LYC] && lyc_interrupt {
            mem.io[IF] = alu::set_bit(mem.io[IF], 1, true);
            mem.io[STAT] = alu::set_bit(mem.io[STAT], 2, true);
        } else {
            mem.io[STAT] = alu::set_bit(mem.io[STAT], 2, false);
        }
    }
    fn colorise(pixel: &Pixel) -> [u8; 3] {
        let palette = pixel.palette;
        let ids = [
            alu::read_bits(palette, 0, 2),
            alu::read_bits(palette, 2, 2),
            alu::read_bits(palette, 4, 2),
            alu::read_bits(palette, 6, 2),
        ];
        match ids[pixel.color_id as usize] {
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
                self.last_cycle = *t_cycles;
                self.mode.stat_interrupt(mem);
                PPU::check_lyc(mem);
                self.current_oam = PPU::fetch_from_oam(mem).ok();
                self.discard_counter = mem.io[SCX] & 7;
                self.mode = PPUMode::Draw(DrawLayer::Bg);
                // 80-4 = 76
                self.cycle_deficit += 76;
            }
            PPUMode::Draw(_) => {
                self.mode.stat_interrupt(mem);
                if self.bg_fifo.is_empty() {
                    for _ in 0..2 {
                        let _ = match self.fetcher.phase {
                            0 => self.fetcher.fetch_bg_tile(mem, self.lx),
                            1 | 2 => self.fetcher.fetch_tile_data(mem, self.lx),
                            3 => {
                                self.fetcher.push_to_fifo(mem, &mut self.bg_fifo);
                                Ok(0)
                            }
                            _ => Ok(0),
                        };
                    }
                }
                if !self.bg_fifo.is_empty() {
                    for _ in 0..4 {
                        let wy = mem.io[WY];
                        let wx = mem.io[WX] as isize - 7;
                        let lcdc = mem.io[LCDC];
                        // TODO: This is disgusting, clean up later please
                        let is_window = alu::read_bits(lcdc, 5, 1) == 1
                            && (wx..(wx + 160)).contains(&(self.lx as isize))
                            && wy <= mem.io[LY];
                        if is_window
                            && let PPUMode::Draw(layer) = &self.mode
                            && *layer != DrawLayer::Window
                        {
                            self.mode = PPUMode::Draw(DrawLayer::Window);
                            self.bg_fifo.clear();
                            break;
                        } else if !is_window {
                            self.mode = PPUMode::Draw(DrawLayer::Bg);
                        }
                        let pixel = self.bg_fifo.pop_front().unwrap();
                        if self.discard_counter == 0 {
                            let framebuffer_index =
                                ((mem.io[LY] as usize * 160) + self.lx as usize) * 3;
                            self.framebuffer[framebuffer_index..framebuffer_index + 3]
                                .copy_from_slice(&PPU::colorise(&pixel));
                            self.lx += 1;
                            if self.lx == 160 {
                                break;
                            }
                        } else {
                            self.discard_counter -= 1;
                        }
                    }
                }
                if self.lx > 159 {
                    self.mode = PPUMode::HBlank;
                }
            }
            PPUMode::HBlank => {
                self.mode.stat_interrupt(mem);
                let draw_len = delta - 80;
                self.cycle_deficit += 456 - draw_len;
                mem.io[LY] = mem.io[LY].wrapping_add(1);
                if mem.io[LY] == 144 {
                    self.mode = PPUMode::VBlank;
                } else {
                    self.mode = PPUMode::Scan;
                }
                self.lx = 0;
                self.bg_fifo.clear();
                trace!("LY: {}", mem.io[LY]);
                trace!("Framebuffer: {:#?}", &self.framebuffer[0..20]);
            }
            PPUMode::VBlank => {
                if mem.io[LY] == 144 {
                    self.mode = PPUMode::VBlank;
                    self.mode.stat_interrupt(mem);
                    self.frame_flag = true;
                    mem.io[IF] = alu::set_bit(mem.io[IF], 0, true);
                    PPU::check_lyc(mem);
                    mem.io[LY] += 1;
                } else if mem.io[LY] == 153 {
                    mem.io[LY] = 0;
                    self.mode = PPUMode::Scan;
                } else {
                    PPU::check_lyc(mem);
                    mem.io[LY] += 1;
                }
                self.cycle_deficit += 456;
            }
        }
    }
    fn fetch_from_oam(mem: &Memory) -> Result<[Option<GBSprite>; 10], GBError> {
        let mut sprite_table = [None; 10];
        let ly = mem.io[LY];
        let mut index = 0;
        for obj_addr in (0xFE00..0xFEA0).step_by(4) {
            let y = (mem.dma_read(obj_addr)? as u16 as i16) - 16;
            let x = (mem.dma_read(obj_addr + 1)? as u16 as i16) - 8;
            let tile_index = mem.dma_read(obj_addr + 2)?;
            let attributes = mem.dma_read(obj_addr + 3)?;
            let obj_size = if alu::read_bits(mem.io[LCDC], 2, 1) == 1 {
                15
            } else {
                7
            };
            if ((ly as isize - obj_size)..=ly as isize).contains(&(y as isize)) && index < 10 {
                sprite_table[index] = Some(GBSprite {
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
        Ok(sprite_table)
    }
    fn fetch_tile_line(mem: &Memory, tile_index: u8, tile_row: u8, is_obj: bool) -> (u8, u8) {
        let lcdc = mem.io[LCDC];
        let base_ptr = if alu::read_bits(lcdc, 4, 1) == 1 || is_obj {
            0x8000
        } else {
            0x9000
        };
        let addr = if base_ptr == 0x8000 {
            base_ptr + (16_usize * tile_index as usize)
        } else {
            (base_ptr as isize + (16_isize * tile_index as i8 as isize)) as usize
        } + (2 * tile_row) as usize;
        let tile_line: (u8, u8) = (mem.dma_read(addr).unwrap(), mem.dma_read(addr + 1).unwrap());
        tile_line
    }
    fn color_from(pixel_color: u8, palette: u8) -> [u8; 3] {
        let ids = [
            alu::read_bits(palette, 0, 2),
            alu::read_bits(palette, 2, 2),
            alu::read_bits(palette, 4, 2),
            alu::read_bits(palette, 6, 2),
        ];
        match ids[pixel_color as usize] {
            0 => [0xFF, 0xFF, 0xFF],
            1 => [0xD3, 0xD3, 0xD3],
            2 => [0x69, 0x69, 0x69],
            3 => [0x0, 0x0, 0x0],
            _ => unreachable!(),
        }
    }
    fn draw_background(&mut self, mem: &Memory) -> Result<(), GBError> {
        let lcdc = mem.io[LCDC];
        let ly: usize = mem.io[LY] as usize;
        let scx = mem.io[SCX] as usize;
        let scy = mem.io[SCY] as usize;
        let mut map_col = scx >> 3;
        let map_row = ((ly + scy) >> 3) & 31;
        let map_addr = if alu::read_bits(lcdc, 3, 1) == 1 {
            0x9C00_usize
        } else {
            0x9800_usize
        } + map_row * 32;
        let pixel_row = ((ly + scy) & 7) as u8;
        let mut tile_index = mem.dma_read(map_addr + map_col)?;
        let mut tile = PPU::fetch_tile_line(mem, tile_index, pixel_row, false);
        for (offset, virtual_index) in (scx..(scx + 160)).enumerate() {
            let tilemap_pixel_index = virtual_index & 255;
            let new_map_col = tilemap_pixel_index >> 3;
            if map_col != new_map_col {
                map_col = new_map_col;
                tile_index = mem.dma_read(map_addr + map_col)?;
                tile = PPU::fetch_tile_line(mem, tile_index, pixel_row, false);
            }
            let curr_tile_pixel = 7 - (tilemap_pixel_index & 7);
            let pixel_color = (alu::read_bits(tile.0, curr_tile_pixel as u8, 1) << 1)
                + alu::read_bits(tile.1, curr_tile_pixel as u8, 1);
            let rgb = PPU::color_from(pixel_color, mem.io[BGP]);
            let framebuffer_index = (ly * 160 + offset) * 3;
            self.framebuffer[framebuffer_index..framebuffer_index + 3].copy_from_slice(&rgb);
        }
        Ok(())
    }
    fn draw_sprites(&mut self, mem: &Memory) -> Result<(), GBError> {
        if self.current_oam.is_none() {
            log::error!("No sprites");
            return Ok(());
        }
        let ly: usize = mem.io[LY] as usize;
        for sprite in self.current_oam.unwrap().into_iter().flatten() {
            let tile_height = if alu::read_bits(mem.io[LCDC], 2, 1) == 1 {
                15
            } else {
                7
            };
            let mut tile_row = (ly as isize - sprite.y as isize) as u8;
            if sprite.y_flip {
                tile_row = tile_height - tile_row;
            }
            let tile_line = PPU::fetch_tile_line(mem, sprite.tile_index, tile_row, true);
            let palette = mem.io[if sprite.dmg_palette == 0 { OBP0 } else { OBP1 }];
            for (offset, x_pos) in (sprite.x..sprite.x + 8).enumerate() {
                if !(0..160).contains(&x_pos) {
                    continue;
                }
                let bit = if sprite.x_flip { offset } else { 7 - offset };
                let pixel_color = (alu::read_bits(tile_line.0, bit as u8, 1) << 1)
                    + alu::read_bits(tile_line.1, bit as u8, 1);
                if pixel_color == 0 {
                    continue;
                }
                let rgb = PPU::color_from(pixel_color, palette);
                let framebuffer_index = (ly * 160 + x_pos as usize) * 3;
                self.framebuffer[framebuffer_index..framebuffer_index + 3].copy_from_slice(&rgb);
            }
        }
        Ok(())
    }
    fn draw_window(&mut self, mem: &Memory) -> Result<(), GBError> {
        let lcdc = mem.io[LCDC];
        let ly: usize = mem.io[LY] as usize;
        let wy = mem.io[WY] as usize;
        let wx = mem.io[WX] as isize - 7;
        let window_y = ly - wy;
        let pixel_row = (window_y & 7) as u8;
        let mut map_col = 0;
        let window_map_addr = if alu::read_bits(lcdc, 6, 1) == 0 {
            0x9800
        } else {
            0x9C00
        } + 32 * (window_y >> 3);
        let mut tile_index = mem.dma_read(window_map_addr)?;
        let mut tile = PPU::fetch_tile_line(mem, tile_index, pixel_row, false);
        for (offset, pixel_index) in (wx..wx + 167).enumerate() {
            if pixel_index > 159 {
                break;
            } else if pixel_index < 0 {
                continue;
            }
            let new_map_col = offset >> 3;
            if new_map_col != map_col {
                map_col = new_map_col;
                tile_index = mem.dma_read(window_map_addr + map_col)?;
                tile = PPU::fetch_tile_line(mem, tile_index, pixel_row, false);
            }
            let curr_tile_pixel = 7 - (offset & 7);
            let pixel_color = (alu::read_bits(tile.0, curr_tile_pixel as u8, 1) << 1)
                + alu::read_bits(tile.1, curr_tile_pixel as u8, 1);
            let rgb = PPU::color_from(pixel_color, mem.io[BGP]);
            let framebuffer_index = (ly * 160 + pixel_index as usize) * 3;
            self.framebuffer[framebuffer_index..framebuffer_index + 3].copy_from_slice(&rgb);
        }
        Ok(())
    }
    fn draw_scanline(&mut self, mem: &Memory) -> Result<(), GBError> {
        let lcdc = mem.io[LCDC];
        let wy = mem.io[WY] as usize;
        let ly: usize = mem.io[LY] as usize;
        if ly == 143 {
            self.frame_flag = true;
        }
        self.draw_background(mem)?;
        if wy <= ly && alu::read_bits(lcdc, 5, 1) == 1 {
            self.draw_window(mem)?;
        }
        self.draw_sprites(mem)?;
        Ok(())
    }
}
