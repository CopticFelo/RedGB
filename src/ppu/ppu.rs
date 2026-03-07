extern crate sdl3;

use log::{debug, info, trace};
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::pixels::{Color, PixelFormat};
use sdl3::render::{FPoint, Texture, TextureCreator, WindowCanvas};
use sdl3::video::WindowContext;
use sdl3::{Error, EventPump, Sdl, VideoSubsystem};

use crate::cpu::alu;
use crate::cpu::cpu_context::CpuContext;
use crate::error::GBError;
use crate::mem::map::MemoryMap;

const LCDC: usize = 0x40;
const LY: usize = 0x44;
const BGP: usize = 0x47;

pub struct PPU {
    pub sdl_context: Sdl,
    video: VideoSubsystem,
    canvas: WindowCanvas,
    event_pump: EventPump,
    last_cycle: u64,
    framebuffer: Vec<u8>,
    texture_creator: TextureCreator<WindowContext>,
    texture: Texture
}

impl PPU {
    pub fn new() -> Result<Self, Error> {
        let sdl_context = sdl3::init()?;
        let video = sdl_context.video()?;
        let window = video.window("RedGB", 256, 256).build().unwrap();
        let mut canvas = window.into_canvas();
        let texture_creator = canvas.texture_creator();
        let 
        canvas.set_draw_color(Color::RGB(149, 171, 18));
        canvas.clear();
        canvas.present();
        let event_pump = sdl_context.event_pump()?;
        Ok(Self {
            sdl_context,
            video,
            canvas,
            event_pump,
            last_cycle: 0,
            framebuffer: vec![0x0; 256 * 256 * 4],
            texture_creator,
        })
    }

    pub fn is_exit(&mut self) -> bool {
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    return true;
                }
                _ => (),
            }
        }
        false
    }
    pub fn tick(context: &mut CpuContext) {
        if context.t_cycles.abs_diff(context.ppu.last_cycle) >= 456 {
            let _ = PPU::draw_scanline(context);
            context.memory.io[LY] = context.memory.io[LY].wrapping_add(1);
            context.ppu.last_cycle = context.t_cycles;
            trace!("LY: {}", context.memory.io[LY]);
            trace!("Framebuffer: {:#?}", &context.ppu.framebuffer[0..20]);
        }
        if context.memory.io[LY] > 153 {
            context.memory.io[LY] = 0;
            PPU::draw_to_sdl(context);
        } else if context.memory.io[LY] == 144 {
            context.memory.io[0x0F] = alu::set_bit(context.memory.io[0x0F], 0, true);
        }
    }
    pub fn draw_to_sdl(context: &mut CpuContext) {
        let mut texture = context
            .ppu
            .texture_creator
            .create_texture_streaming(PixelFormat::RGBA8888, 256, 256)
            .unwrap();
        texture
            .update(None, &context.ppu.framebuffer[..], 256 * 4)
            .unwrap();
        context.ppu.canvas.clear();
        context.ppu.canvas.copy(&texture, None, None).unwrap();
        context.ppu.canvas.present();
    }
    pub fn fetch_tile_line(
        context: &mut CpuContext,
        base_ptr: usize,
        tile_index: u8,
        index: u8,
    ) -> (u8, u8) {
        let addr = if base_ptr == 0x8000 {
            base_ptr + (16_usize * tile_index as usize)
        } else {
            (base_ptr as isize + (16_isize * tile_index as isize)) as usize
        } + (2 * index) as usize;
        let tile_line: (u8, u8) = (
            MemoryMap::dma_read(context, addr).unwrap(),
            MemoryMap::dma_read(context, addr + 1).unwrap(),
        );
        tile_line
    }
    pub fn color_from_bgb(pixel_color: u8, context: &mut CpuContext) -> [u8; 4] {
        let bgb = context.memory.io[BGP];
        let ids = [
            alu::read_bits(bgb, 0, 2),
            alu::read_bits(bgb, 2, 2),
            alu::read_bits(bgb, 4, 2),
            alu::read_bits(bgb, 6, 2),
        ];
        match ids[pixel_color as usize] {
            0 => [0xFF, 0xFF, 0xFF, 0xFF],
            1 => [0xD3, 0xD3, 0xD3, 0xFF],
            2 => [0x69, 0x69, 0x69, 0xFF],
            3 => [0x00, 0x00, 0x00, 0xFF],
            _ => unreachable!(),
        }
    }
    pub fn draw_scanline(context: &mut CpuContext) -> Result<(), GBError> {
        let lcdc = context.memory.io[LCDC];
        let ly = context.memory.io[LY];
        let base_ptr = if alu::read_bits(lcdc, 4, 1) == 1 {
            0x8000
        } else {
            0x9000
        };
        let map_addr = if alu::read_bits(lcdc, 3, 1) == 1 {
            0x9C00_usize
        } else {
            0x9800_usize
        } + 32_usize * (ly as usize / 8);
        for i in 0..32 {
            let tile_index = MemoryMap::dma_read(context, map_addr + i)?;
            let tile = PPU::fetch_tile_line(context, base_ptr, tile_index, ly % 8);
            for j in (0..8).rev() {
                let pixel_color =
                    (alu::read_bits(tile.0, j, 1) << 1) + alu::read_bits(tile.1, j, 1);
                let rgba = PPU::color_from_bgb(pixel_color, context);
                let framebuffer_index = (ly as usize) * 256 + (7 - j as usize);
                context
                    .ppu
                    .framebuffer
                    .splice(framebuffer_index..framebuffer_index, rgba);
            }
        }
        Ok(())
    }
}
