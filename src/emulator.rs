use std::time::{Duration, Instant};

use log::{debug, info};
use sdl3::EventPump;
use sdl3::event::Event;
use sdl3::gpu::Buffer;
use sdl3::keyboard::Keycode;
use sdl3::pixels::PixelFormat;
use sdl3::render::{TextureCreator, WindowCanvas};
use sdl3::sys::render::SDL_RendererLogicalPresentation;
use sdl3::video::WindowContext;

use crate::cpu::cpu_context::CpuContext;
use crate::cpu::reg_file::{Modes, RegFile};
use crate::error::GBError;
use crate::mem::map;
use crate::ppu::ppu::PPU;
use crate::rom::rom_info::ROMInfo;

pub fn init_emulation(rom: Vec<u8>, header_data: ROMInfo) -> Result<(), GBError> {
    // Init SDL
    let sdl_context = sdl3::init().expect("Error: Could not init SDL");
    let video = sdl_context
        .video()
        .expect("Error: Could not init SDL Video subsystem");
    let window = video
        .window("RedGB", 800, 720)
        .build()
        .expect("Error: Could not display window");
    let mut canvas = window.into_canvas();
    canvas
        .set_logical_size(160, 144, SDL_RendererLogicalPresentation::INTEGER_SCALE)
        .unwrap();
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormat::RGB24, 160, 144)
        .expect("Error: Could not create streaming texture");
    let mut event_pump = sdl_context
        .event_pump()
        .expect("Error: Could not capture game input");
    let registers = RegFile::new(Modes::DMG);
    let memory = map::MemoryMap::init_rom(rom, header_data);
    let ppu = PPU::new();
    let mut context = CpuContext::init(registers, memory, ppu);
    context.memory.io[0x0] = 255;
    let mut time = Instant::now();
    let target = Duration::new(0, 16666667);
    loop {
        context.step()?;
        if time.elapsed() < target {
            std::thread::sleep(target.abs_diff(time.elapsed()));
        }
        let fps = 1.0 / (time.elapsed().as_secs_f32());
        debug!("fps: {}", fps);
        time = Instant::now();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    info!("Cycle count: {}", &context.t_cycles);
                    info!("CPU {:#?}", &context.registers);
                    info!("Last Serial message: {}", {
                        str::from_utf8(&context.serial_message[..]).unwrap()
                    });
                    return Ok(());
                }
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    context.handle_joypad(key, true);
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    context.handle_joypad(key, false);
                }
                _ => (),
            }
        }
        texture
            .with_lock(None, |buffer: &mut [u8], _: usize| {
                buffer.copy_from_slice(context.ppu.framebuffer.as_slice());
            })
            .unwrap();
        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
    }
}
