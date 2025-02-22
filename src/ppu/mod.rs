use crate::{memory::Memory, SCREEN_HEIGHT, SCREEN_WIDTH};

mod bitmaps;
mod tiles;

use bitmaps::*;
use tiles::*;

const VRAM_BASE: u32 = 0x6000000;
const PALETTE_BASE: u32 = 0x5000000;

fn convert_palette_winit(palette: u16) -> u32 {
    let (r, g, b) = (palette & 0x1F, (palette >> 5) & 0x1F, (palette >> 10) & 0x1F);
    let (float_r, float_g, float_b) = (r as f32 / 31., g as f32 / 31., b as f32 / 31.);
    let (pixel_r, pixel_g, pixel_b) = (float_r * 255., float_g * 255., float_b * 255.);
    let color = (pixel_r as u32) << 16 | (pixel_g as u32) << 8 | (pixel_b as u32);
    return color 
}
enum PpuRegisters {
    Dispcnt = 0x4000000,
    _GreenSwap = 0x4000002,
    DispStat = 0x4000004,
    VCount = 0x4000006,
    BGCnt = 0x4000008,
    BgHOffset = 0x4000010,
    BgVOffset = 0x4000012,
    _Mosaic = 0x400004C,
}

pub struct Ppu {
    pub new_screen: bool,
    elapsed_time: usize, // represents the number of dots elapsed
    pub stored_screen: Vec<u32>,
    in_hblank: bool,
    in_vblank: bool,
}
impl Ppu {
    pub fn new() -> Self {
        Self { 
            new_screen: false,
            elapsed_time: 0,
            stored_screen: Vec::new(),
            in_hblank: false,
            in_vblank: false,
        }
    }
    pub fn acknowledge_frame(&mut self) {
        self.new_screen = false;
        self.elapsed_time = 0;
        self.stored_screen = Vec::new();
    }
}
const DOTS_PER_FRAME: usize = (SCREEN_WIDTH + 68) * (SCREEN_HEIGHT + 68);
pub fn tick_ppu(ppu: &mut Ppu, memory: &mut Memory) {
    let dispcnt = memory.read_u16(PpuRegisters::Dispcnt as u32);
    let forced_blank = (dispcnt >> 7) & 1 == 1;
    if forced_blank {
        return;
    }

    // the line count we had last time, doesnt match the one this time
    let dispstat = memory.read_u16(PpuRegisters::DispStat as u32);
    let mut vcount = memory.read_u16(PpuRegisters::VCount as u32);

    let new_line = ppu.elapsed_time / (SCREEN_WIDTH + 68) != vcount as usize;
    if new_line {
        vcount += 1;

        if vcount < SCREEN_HEIGHT as u16 {
            let bg_mode = dispcnt & 0b111;
            match bg_mode {
                0 => bg_mode_0(ppu, memory, vcount as u32),
                1 => bg_mode_1(ppu, memory, vcount),
                2 => bg_mode_2(ppu, memory, vcount),
                3 => bg_mode_3(ppu, memory, vcount),
                4 => bg_mode_4(ppu, memory, vcount),
                5 => bg_mode_5(ppu, memory, vcount),
                _ => panic!("you can't set the bg_mode to {bg_mode}"),
            }
        }

        if vcount as usize >= (SCREEN_HEIGHT + 68) {
            vcount = 0;
            ppu.new_screen = true;
        }

        update_registers(ppu, memory, dispstat, vcount);
        memory.write_io(PpuRegisters::VCount as u32, vcount as u16);
    }
}
fn update_registers(ppu: &mut Ppu, memory: &mut Memory, mut dispstat: u16, vcount: u16) {
    // work in progress
    ppu.elapsed_time += 1;
    
    // new frame
    if ppu.elapsed_time >= DOTS_PER_FRAME {
        ppu.elapsed_time = 0;
        dispstat &= !(0b11<<0);
    }

    // V-blank flag
    let in_vblank = ppu.elapsed_time / (SCREEN_WIDTH + 68) >= SCREEN_HEIGHT;
    if in_vblank != ppu.in_vblank {
        match in_vblank {
            true => dispstat |= 1<<0,
            false => dispstat &= !(1<<0),
        }  
        ppu.in_vblank = in_vblank
    }

    // H-blank flag
    let in_hblank = ppu.elapsed_time % (SCREEN_WIDTH + 68) >= SCREEN_WIDTH;
    if in_hblank != ppu.in_hblank {
        match in_hblank {
            true => dispstat |= 1<<1,
            false => dispstat &= !(1<<1),
        }
        ppu.in_hblank = in_hblank;
    }

    let vcount_lyc = (dispstat >> 8) & 0xFF;
    let vcounter_match = vcount == vcount_lyc;
    match vcounter_match {
        true => dispstat |= 1<<2,
        false => dispstat &= !(1<<2),
    }  

    let mut ie = memory.read_u16(0x4000202);
    ie &= !0b111;
    ie |= dispstat & 0b111;

    memory.write_io(0x4000202, ie);
    memory.write_io(PpuRegisters::DispStat as u32, dispstat);
}

