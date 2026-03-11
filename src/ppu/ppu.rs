extern crate sdl3;

use log::{debug, info, trace};

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
            }
            context.memory.io[LY] = context.memory.io[LY].wrapping_add(1);
            context.ppu.last_cycle = context.t_cycles;
            trace!("LY: {}", context.memory.io[LY]);
            trace!("Framebuffer: {:#?}", &context.ppu.framebuffer[0..20]);
        }
        let lyc_interrupt = alu::read_bits(context.memory.io[STAT], 6, 1) == 1;
        if context.memory.io[LY] == context.memory.io[LYC] && lyc_interrupt {
            alu::set_bit(context.memory.io[IF], 1, true);
            alu::set_bit(context.memory.io[STAT], 2, true);
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
            let first_visible = if y < 0 { 0 } else { y as u8 };
            let tile_index = MemoryMap::dma_read(context, obj_addr + 2)?;
            let attributes = MemoryMap::dma_read(context, obj_addr + 3)?;
            if (ly.saturating_sub(7)..=ly).contains(&first_visible) && index < 10 {
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
    pub fn color_from_bgb(pixel_color: u8, context: &mut CpuContext) -> [u8; 3] {
        let bgb = context.memory.io[BGP];
        let ids = [
            alu::read_bits(bgb, 0, 2),
            alu::read_bits(bgb, 2, 2),
            alu::read_bits(bgb, 4, 2),
            alu::read_bits(bgb, 6, 2),
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
        let scy = context.memory.io[SCY] as usize;
        let map_col = (scx >> 3);
        let map_row = ((ly + scy) >> 3) & 31;
        let map_addr = if alu::read_bits(lcdc, 3, 1) == 1 {
            0x9C00_usize
        } else {
            0x9800_usize
        } + map_row * 32;
        // i is a map horizontal offset
        let mut i = map_col;
        // Loop over 20 tiles in tile map (each scanline is 20 tiles across at most)
        // be careful of boundries because scx offset
        let mut pixels_drawn = 0;
        for index in 0..20 {
            // get tile index for tile map using (map_addr + i)
            let tile_index = MemoryMap::dma_read(context, map_addr + i)?;
            // fetch the 2 bytes that form a line in a tile
            // the line we want is calculated by (ly + scy) % 8
            // because ly and scy are pixel offsets
            let tile = PPU::fetch_tile_line(
                context,
                tile_index,
                ((ly as u8).wrapping_add(scy as u8)) & 7,
                false,
            );
            // calculate pixel offsets because of scx
            let pixel_start = if i == 0 { 8 - (scx & 7) } else { 8 };
            let pixel_end = if i == 19 { scx & 7 } else { 0 };
            // fill the next 8 bits with pixel data
            for j in (pixel_end..pixel_start).rev() {
                let pixel_color =
                    (alu::read_bits(tile.0, j as u8, 1) << 1) + alu::read_bits(tile.1, j as u8, 1);
                let rgb = PPU::color_from_bgb(pixel_color, context);
                let framebuffer_index = (ly * 160 + (7 - j) + pixels_drawn) * 3;
                context.ppu.framebuffer[framebuffer_index..framebuffer_index + 3]
                    .copy_from_slice(&rgb);
            }
            pixels_drawn += 8;
            i = (i + 1) & 31;
        }
        // Draw sprites
        let sprite_table = PPU::fetch_from_oam(context)?;
        for sprite in sprite_table.into_iter().flatten() {
            let mut tile_row = (ly.wrapping_sub_signed(sprite.y as isize) as u8) & 7;
            if sprite.y_flip {
                tile_row = 7 - tile_row;
            }
            let tile_line = PPU::fetch_tile_line(context, sprite.tile_index, tile_row, true);
            let (mut pixel_start, first_visible) = if sprite.x < 0 {
                ((8 + sprite.x) as u8, 0)
            } else {
                (8, sprite.x as usize)
            };
            let mut pixel_end = (sprite.x as u8).saturating_sub(160);
            if sprite.x_flip {
                std::mem::swap(&mut pixel_end, &mut pixel_start);
            }
            for bit in (pixel_end..pixel_start).rev() {
                let pixel_color = (alu::read_bits(tile_line.0, bit, 1) << 1)
                    + alu::read_bits(tile_line.1, bit, 1);
                let rgb = PPU::color_from_bgb(pixel_color, context);
                let framebuffer_index = (ly * 160 + first_visible + (7 - bit as usize)) * 3;
                context.ppu.framebuffer[framebuffer_index..framebuffer_index + 3]
                    .copy_from_slice(&rgb);
            }
        }
        Ok(())
    }
}
