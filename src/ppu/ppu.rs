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
                } else {
                    self.fifo_pop(mem);
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
    fn fifo_pop(&mut self, mem: &Memory) {
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
                let framebuffer_index = ((mem.io[LY] as usize * 160) + self.lx as usize) * 3;
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
}
