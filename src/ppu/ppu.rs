extern crate sdl3;

use itertools::Either;
use log::trace;

use crate::cpu::alu;
use crate::cpu::cpu_context::CpuContext;
use crate::error::GBError;
use crate::mem::map::MemoryMap;
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

pub struct PPU {
    last_cycle: u64,
    pub framebuffer: Vec<u8>,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            last_cycle: 0,
            framebuffer: vec![0x0; 160 * 144 * 3],
        }
    }

    pub fn tick(context: &mut CpuContext) {
        if context.t_cycles.abs_diff(context.ppu.last_cycle) >= 456 {
            if context.memory.io[LY] < 144 {
                let _ = PPU::draw_scanline(context);
                context.memory.io[IF] = alu::set_bit(context.memory.io[IF], 1, true);
                context.memory.io[STAT] = alu::set_bit(context.memory.io[STAT], 3, true);
            }
            context.memory.io[LY] = context.memory.io[LY].wrapping_add(1);
            context.ppu.last_cycle = context.t_cycles;
            trace!("LY: {}", context.memory.io[LY]);
            trace!("Framebuffer: {:#?}", &context.ppu.framebuffer[0..20]);
        }
        let lyc_interrupt = alu::read_bits(context.memory.io[STAT], 6, 1) == 1;
        if context.memory.io[LY] == context.memory.io[LYC] && lyc_interrupt {
            context.memory.io[IF] = alu::set_bit(context.memory.io[IF], 1, true);
            context.memory.io[STAT] = alu::set_bit(context.memory.io[STAT], 2, true);
        } else if context.memory.io[LY] > 153 {
            context.memory.io[LY] = 0;
        } else if context.memory.io[LY] == 144 {
            context.memory.io[0x0F] = alu::set_bit(context.memory.io[0x0F], 0, true);
        }
    }
    pub fn fetch_from_oam(context: &mut CpuContext) -> Result<[Option<GBSprite>; 10], GBError> {
        let mut sprite_table = [None; 10];
        let ly = context.memory.io[LY];
        let mut index = 0;
        for obj_addr in (0xFE00..0xFEA0).step_by(4) {
            let y = (MemoryMap::dma_read(context, obj_addr)? as u16 as i16) - 16;
            let x = (MemoryMap::dma_read(context, obj_addr + 1)? as u16 as i16) - 8;
            let tile_index = MemoryMap::dma_read(context, obj_addr + 2)?;
            let attributes = MemoryMap::dma_read(context, obj_addr + 3)?;
            let obj_size = if alu::read_bits(context.memory.io[LCDC], 2, 1) == 1 {
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
    pub fn fetch_tile_line(
        context: &mut CpuContext,
        tile_index: u8,
        tile_row: u8,
        is_obj: bool,
    ) -> (u8, u8) {
        let lcdc = context.memory.io[LCDC];
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
        let tile_line: (u8, u8) = (
            MemoryMap::dma_read(context, addr).unwrap(),
            MemoryMap::dma_read(context, addr + 1).unwrap(),
        );
        tile_line
    }
    pub fn color_from(pixel_color: u8, palette: u8) -> [u8; 3] {
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
    pub fn draw_scanline(context: &mut CpuContext) -> Result<(), GBError> {
        let lcdc = context.memory.io[LCDC];
        let ly: usize = context.memory.io[LY] as usize;
        if ly == 143 {
            context.frame_drawn = true;
        }
        let scx = context.memory.io[SCX] as usize;
        log::info!("SCX: {}", scx);
        let scy = context.memory.io[SCY] as usize;
        let mut map_col = scx >> 3;
        let map_row = ((ly + scy) >> 3) & 31;
        let map_addr = if alu::read_bits(lcdc, 3, 1) == 1 {
            0x9C00_usize
        } else {
            0x9800_usize
        } + map_row * 32;
        let pixel_row = ((ly + scy) & 7) as u8;
        if scx > 0 {
            print!("");
        }
        let mut tile_index = MemoryMap::dma_read(context, map_addr + map_col)?;
        let mut tile = PPU::fetch_tile_line(context, tile_index, pixel_row, false);
        for (offset, virtual_index) in (scx..(scx + 160)).enumerate() {
            let tilemap_pixel_index = virtual_index & 255;
            let new_map_col = tilemap_pixel_index >> 3;
            if map_col != new_map_col {
                map_col = new_map_col;
                tile_index = MemoryMap::dma_read(context, map_addr + map_col)?;
                tile = PPU::fetch_tile_line(context, tile_index, pixel_row, false);
            }
            let curr_tile_pixel = 7 - (tilemap_pixel_index & 7);
            let pixel_color = (alu::read_bits(tile.0, curr_tile_pixel as u8, 1) << 1)
                + alu::read_bits(tile.1, curr_tile_pixel as u8, 1);
            let rgb = PPU::color_from(pixel_color, context.memory.io[BGP]);
            let framebuffer_index = (ly * 160 + offset) * 3;
            context.ppu.framebuffer[framebuffer_index..framebuffer_index + 3].copy_from_slice(&rgb);
        }
        let sprite_table = PPU::fetch_from_oam(context)?;
        for sprite in sprite_table.into_iter().flatten() {
            let tile_height = if alu::read_bits(context.memory.io[LCDC], 2, 1) == 1 {
                15
            } else {
                7
            };
            let mut tile_row = (ly as isize - sprite.y as isize) as u8;
            if sprite.y_flip {
                tile_row = tile_height - tile_row;
            }
            let tile_line = PPU::fetch_tile_line(context, sprite.tile_index, tile_row, true);
            let (pixel_start, first_visible) = if sprite.x < 0 {
                ((8 + sprite.x) as u8, 0)
            } else {
                (8, sprite.x as usize)
            };
            let pixel_end = (sprite.x as u8).saturating_sub(160);
            let draw_range = if sprite.x_flip {
                Either::Left(pixel_end..pixel_start)
            } else {
                Either::Right((pixel_end..pixel_start).rev())
            };
            let palette = context.memory.io[if sprite.dmg_palette == 0 { OBP0 } else { OBP1 }];
            for (offest, bit) in draw_range.enumerate() {
                if bit > 7 {
                    continue;
                }
                let pixel_color = (alu::read_bits(tile_line.0, bit, 1) << 1)
                    + alu::read_bits(tile_line.1, bit, 1);
                if pixel_color == 0 {
                    continue;
                }
                let rgb = PPU::color_from(pixel_color, palette);
                let framebuffer_index = (ly * 160 + first_visible + offest) * 3;
                context.ppu.framebuffer[framebuffer_index..framebuffer_index + 3]
                    .copy_from_slice(&rgb);
            }
        }
        Ok(())
    }
}
