mod bitmaps;
mod tiles;
mod obj;
mod accumulate;

use crate::{memory::Memory, ppu::accumulate::{accumulate_and_palette, LineLayers}};
use bitmaps::*;
use obj::oam_scan;
use tiles::*;

fn _get_rotation_scaling(bg: u32, memory: &Box<Memory>) -> (u32, u32, u16, u16, u16, u16) {
    let base = PpuRegisters::BgRotationBase as u32 + (bg - 2) * 0x10;
    let x0 = {
        let lower = memory.read_u16_io(base + 0x8) as u32;
        let higher = memory.read_u16_io(base + 0xA) as u32 & 0x0FFF;
        higher << 16 | lower
    };
    let y0 = {
        let lower = memory.read_u16_io(base + 0xC) as u32;
        let higher = memory.read_u16_io(base + 0xE) as u32 & 0x0FFF;
        higher << 16 | lower
    };

    let dx = memory.read_u16_io(base + 0x0);
    let dmx = memory.read_u16_io(base + 0x2);
    let dy = memory.read_u16_io(base + 0x4);
    let dmy = memory.read_u16_io(base + 0x6);

    return (x0, y0, dx, dmx, dy, dmy);
}
fn _blend_pixels(x1: u16, x2: u16, eva: f32, evb: f32) -> u16 {
    let (old_r, old_g, old_b) = (x2 & 0x1F, (x2 >> 5) & 0x1F, (x2 >> 10) & 0x1F);
    let (new_r, new_g, new_b) = (x1 & 0x1F, (x1 >> 5) & 0x1F, (x1 >> 10) & 0x1F);

    let mut combo_r = new_r as f32 * eva + old_r as f32 * evb;
    if combo_r > 31. {
        combo_r = 31.;
    }
    let mut combo_g = new_g as f32 * eva + old_g as f32 * evb;
    if combo_g > 31. {
        combo_g = 31.;
    }
    let mut combo_b = new_b as f32 * eva + old_b as f32 * evb;
    if combo_b > 31. {
        combo_b = 31.;
    }

    let combined_pixel = (combo_r as u16) | (combo_g as u16) << 5 | (combo_b as u16) << 10;
    return combined_pixel;
}

const LCD_HEIGHT: usize = 160;
const LCD_WIDTH: usize = 240;
const VRAM_BASE: u32 = 0x6000000;
const PALETTE_BASE: u32 = 0x5000000;
const DOTS_PER_FRAME: usize = (LCD_WIDTH + 68) * (LCD_HEIGHT + 68);

enum PpuRegisters {
    DispCnt = 0x4000000,
    _GreenSwap = 0x4000002,
    DispStat = 0x4000004,
    VCount = 0x4000006,
    BGCnt = 0x4000008,
    BgHOffset = 0x4000010,
    BgVOffset = 0x4000012,
    BgRotationBase = 0x4000020,
    _Win0H = 0x4000040,
    _Win0V = 0x4000044,
    _WinIn = 0x4000048,
    _WinOut = 0x400004A,
    _Mosaic = 0x400004C,
    _BldCnt = 0x4000050,
    _BldAlpha = 0x4000052,
    _BldY = 0x4000054,
}
pub struct Ppu {
    pub new_screen: bool,
    elapsed_time: usize, // represents the number of dots elapsed
    pub stored_screen: Vec<u16>,
}
impl Ppu {
    pub fn new() -> Self {
        Self { 
            new_screen: false,
            elapsed_time: 0,
            stored_screen: Vec::new(),
        }
    }
    pub fn acknowledge_frame(&mut self) {
        self.new_screen = false;
        self.elapsed_time = 0;
        self.stored_screen.clear();
    }
}

pub fn tick_ppu(ppu: &mut Ppu, memory: &mut Box<Memory>) {
    let dispcnt = memory.read_u16_io(PpuRegisters::DispCnt as u32);

    // the line count we had last time, doesnt match the one this time
    let dispstat = memory.read_u16_io(PpuRegisters::DispStat as u32);
    let mut vcount = memory.read_u16_io(PpuRegisters::VCount as u32);

    let new_line = ppu.elapsed_time / (LCD_WIDTH + 68) != vcount as usize;
    if new_line {
        if vcount < LCD_HEIGHT as u16 {
            let mut layers = LineLayers::blank();

            // clear it for the new line
            let bg_mode = dispcnt & 0b111;
            match bg_mode {
                0 => bg_mode_0(&mut layers, memory, vcount as u32),
                1 => bg_mode_1(&mut layers, memory, vcount as u32),
                2 => bg_mode_2(&mut layers, memory, vcount),
                3 => bg_mode_3(&mut layers, memory, vcount),
                4 => bg_mode_4(&mut layers, memory, vcount),
                5 => bg_mode_5(&mut layers, memory, vcount),
                _ => panic!("you can't set the bg_mode to {bg_mode}"),
            };
            oam_scan(&mut layers, memory, vcount, dispcnt);

            let combo = accumulate_and_palette(&layers, &memory);
            ppu.stored_screen.extend(combo);
        }
        vcount += 1;
        if vcount as usize >= (LCD_HEIGHT + 68) {
            vcount = 0;
            ppu.new_screen = true;
        }
        memory.write_io(PpuRegisters::VCount as u32, vcount as u16);
    }

    update_registers(ppu, memory, dispstat, vcount);
}

fn update_registers(ppu: &mut Ppu, memory: &mut Box<Memory>, mut dispstat: u16, vcount: u16) {
    // work in progress
    ppu.elapsed_time += 1;
    
    // new frame
    if ppu.elapsed_time >= DOTS_PER_FRAME {
        ppu.elapsed_time = 0;
        dispstat &= !(0b11<<0);
    }

    // V-blank flag
    let in_vblank = ppu.elapsed_time / (LCD_WIDTH + 68) >= LCD_HEIGHT;
    match in_vblank {
        true => dispstat |= 1<<0,
        false => dispstat &= !(1<<0),
    }

    // H-blank flag
    let in_hblank = ppu.elapsed_time % (LCD_WIDTH + 68) >= LCD_WIDTH;
    match in_hblank {
        true => dispstat |= 1<<1,
        false => dispstat &= !(1<<1),
    }

    let vcount_lyc = (dispstat >> 8) & 0xFF;
    let vcounter_match = vcount == vcount_lyc;
    match vcounter_match {
        true => dispstat |= 1<<2,
        false => dispstat &= !(1<<2),
    }  

    let mut ie = memory.read_u16_io(0x4000202);
    ie &= !0b111;
    ie |= dispstat & 0b111;

    memory.write_io(0x4000202, ie);
    memory.write_io(PpuRegisters::DispStat as u32, dispstat);
}