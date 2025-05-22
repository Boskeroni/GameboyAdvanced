use crate::{memory::Memory, SCREEN_HEIGHT, SCREEN_WIDTH};

mod bitmaps;
mod tiles;
mod obj;
mod window;

use window::window_line;
use bitmaps::*;
use obj::oam_scan;
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
    DispCnt = 0x4000000,
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

    pixel_priorities: Vec<u16>, // the priorities of all the pixels on the screen
    worked_on_line: [u16; 240],

}
impl Ppu {
    pub fn new() -> Self {
        Self { 
            new_screen: false,
            elapsed_time: 0,
            stored_screen: Vec::new(),

            pixel_priorities: Vec::new(),
            worked_on_line: [0; 240],
        }
    }
    pub fn acknowledge_frame(&mut self) {
        self.new_screen = false;
        self.elapsed_time = 0;
        self.pixel_priorities.clear();
        self.stored_screen.clear();
    }
}
const DOTS_PER_FRAME: usize = (SCREEN_WIDTH + 68) * (SCREEN_HEIGHT + 68);
pub fn tick_ppu(ppu: &mut Ppu, memory: &mut Memory) {
    let dispcnt = memory.read_u16(PpuRegisters::DispCnt as u32);
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
            // clear it for the new line
            ppu.worked_on_line = [0; 240];
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

            oam_scan(ppu, memory, vcount, dispcnt);
            window_line();

            // should probably just have an accumulate step
            let new_line: Vec<u32> = ppu.worked_on_line.iter().map(
                |c| convert_palette_winit(*c)
            ).collect();
            ppu.stored_screen.extend(new_line);
        }

        if vcount as usize >= (SCREEN_HEIGHT + 68) {
            vcount = 0;
            ppu.new_screen = true;
        }
        memory.write_io(PpuRegisters::VCount as u32, vcount as u16);
          
    }

    update_registers(ppu, memory, dispstat, vcount);
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
    match in_vblank {
        true => dispstat |= 1<<0,
        false => dispstat &= !(1<<0),
    }

    // H-blank flag
    let in_hblank = ppu.elapsed_time % (SCREEN_WIDTH + 68) >= SCREEN_WIDTH;
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


    let mut ie = memory.read_u16(0x4000202);
    ie &= !0b111;
    ie |= dispstat & 0b111;

    memory.write_io(0x4000202, ie);
    memory.write_io(PpuRegisters::DispStat as u32, dispstat);
}

