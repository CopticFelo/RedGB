extern crate sdl3;

use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::pixels::Color;
use sdl3::render::{FPoint, WindowCanvas};
use sdl3::sys::pixels;
use sdl3::{Error, EventPump, Sdl, VideoSubsystem};

use crate::cpu::alu;
use crate::cpu::cpu_context::CpuContext;

const LY: usize = 0x44;

pub struct PPU {
    pub sdl_context: Sdl,
    video: VideoSubsystem,
    canvas: WindowCanvas,
    event_pump: EventPump,
    framebuffer: Vec<Vec<u32>>,
}

impl PPU {
    pub fn new() -> Result<Self, Error> {
        let sdl_context = sdl3::init()?;
        let video = sdl_context.video()?;
        let window = video.window("RedGB", 144, 160).build().unwrap();
        let mut canvas = window.into_canvas();
        canvas.set_draw_color(Color::RGB(149, 171, 18));
        canvas.clear();
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.draw_line(FPoint::new(1.0, 1.0), FPoint::new(80.0, 80.0));
        canvas.present();
        let event_pump = sdl_context.event_pump()?;
        Ok(Self {
            sdl_context,
            video,
            canvas,
            event_pump,
            framebuffer: vec![vec![0x95AB12FF; 144]; 160],
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
        if context.t_cycles.is_multiple_of(456) {
            // TODO: Draw line to framebuffer
            context.memory.io[LY] = context.memory.io[LY].wrapping_add(1);
        }
        if context.memory.io[LY] > 153 {
            // Reset LY
            context.memory.io[LY] = 0;
            // Raise V-Blank interrupt
            context.memory.io[0x0F] = alu::set_bit(context.memory.io[0x0F], 0, true);
            // TODO: Draw framebuffer to SDL
        }
    }
}
