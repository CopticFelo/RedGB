use std::time::{Duration, Instant};

use log::{debug, info};
use ringbuf::HeapRb;
use ringbuf::traits::Split;
use sdl3::audio::{AudioFormat, AudioSpec};
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::pixels::PixelFormat;
use sdl3::render::ScaleMode;
use sdl3::sys::render::SDL_RendererLogicalPresentation;

use crate::apu::buffer;
use crate::bus::Bus;
use crate::cpu::reg_file::{Modes, RegFile};
use crate::cpu::sm83::SM83;
use crate::error::GBError;
use crate::mem::map;
use crate::ppu::ppu::PPU;
use crate::rom::rom_info::ROMInfo;

const GB_AUDIO_SPEC: AudioSpec = AudioSpec {
    freq: Some(1048576),
    channels: Some(1),
    format: Some(AudioFormat::f32_sys()),
};
const AUDIO_SPEC: AudioSpec = AudioSpec {
    freq: Some(44100),
    channels: Some(1),
    format: Some(AudioFormat::f32_sys()),
};

pub fn init_emulation(rom: Vec<u8>, header_data: ROMInfo) -> Result<(), GBError> {
    // Init SDL
    let sdl_bus = sdl3::init().expect("Error: Could not init SDL");
    let video = sdl_bus
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
    texture.set_scale_mode(ScaleMode::Nearest);
    let mut event_pump = sdl_bus
        .event_pump()
        .expect("Error: Could not capture game input");
    let registers = RegFile::new(Modes::DMG);
    let memory = map::MemoryMap::init_rom(rom, header_data);
    let ppu = PPU::new();
    let mut time = Instant::now();
    let target = Duration::new(0, 16666667);
    let audio_sys = sdl_bus.audio().expect("Error: Could not init audio");
    let audio_buf = HeapRb::<f32>::new(2048);
    let (prod, cons) = audio_buf.split();
    let callback_struct = buffer::AudioBuffer { buffer: cons };
    let device = audio_sys
        .open_playback_stream(&AUDIO_SPEC, callback_struct)
        .expect("Error: Could not open audio device");
    let mut bus = Bus::init(registers, memory, ppu, prod);
    bus.apu.tick(&bus.memory);
    bus.memory.io[0x0] = 255;
    device.resume().expect("Error: couldn't start playback");
    loop {
        SM83::step(&mut bus)?;
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
                    device.pause();
                    info!("Cycle count: {}", &bus.t_cycles);
                    info!("CPU {:#?}", &bus.registers);
                    info!("Audio: {:#?}", &bus.memory.io[0x10..=0x26]);
                    info!("Last Serial message: {}", {
                        str::from_utf8(&bus.serial_message[..]).unwrap()
                    });
                    return Ok(());
                }
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    bus.joypad.update(key, true);
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    bus.joypad.update(key, false);
                }
                _ => (),
            }
        }
        texture
            .with_lock(None, |buffer: &mut [u8], _: usize| {
                buffer.copy_from_slice(bus.ppu.framebuffer.as_slice());
            })
            .unwrap();
        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
    }
}
