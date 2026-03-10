extern crate sdl3;

use log::{debug, info, trace};

use crate::cpu::alu;
use crate::cpu::cpu_context::CpuContext;
use crate::error::GBError;
use crate::mem::map::MemoryMap;

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
    pub fn fetch_tile_line(context: &mut CpuContext, tile_index: u8, tile_row: u8) -> (u8, u8) {
        let lcdc = context.memory.io[LCDC];
        let base_ptr = if alu::read_bits(lcdc, 4, 1) == 1 {
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
        let map_col = (scx >> 3) & 31;
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
        Ok(())
    }
}
