mod bitmaps;
mod tiles;
mod obj;
mod window;

use crate::memory::Memory;
use window::window_line;
use bitmaps::*;
use obj::oam_scan;
use tiles::*;

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
    _Mosaic = 0x400004C,
}
pub struct Ppu {
    pub new_screen: bool,
    elapsed_time: usize, // represents the number of dots elapsed
    pub stored_screen: Vec<u16>,
    worked_on_line: [u16; 240],
}
impl Ppu {
    pub fn new() -> Self {
        Self { 
            new_screen: false,
            elapsed_time: 0,
            stored_screen: Vec::new(),

            worked_on_line: [0; 240],
        }
    }
    pub fn acknowledge_frame(&mut self) {
        self.new_screen = false;
        self.elapsed_time = 0;
        self.stored_screen.clear();
    }
}

fn get_rotation_scaling(bg: u32, memory: &Box<Memory>) -> (u32, u32, u16, u16, u16, u16) {
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

pub fn tick_ppu(ppu: &mut Ppu, memory: &mut Box<Memory>) {
    let dispcnt = memory.read_u16_io(PpuRegisters::DispCnt as u32);

    // the line count we had last time, doesnt match the one this time
    let dispstat = memory.read_u16_io(PpuRegisters::DispStat as u32);
    let mut vcount = memory.read_u16_io(PpuRegisters::VCount as u32);

    let new_line = ppu.elapsed_time / (LCD_WIDTH + 68) != vcount as usize;
    if new_line {
        if vcount < LCD_HEIGHT as u16 {
            // clear it for the new line
            ppu.worked_on_line = [0; LCD_WIDTH];
            let bg_mode = dispcnt & 0b111;
            let (bg_scan, bg_prio) = match bg_mode {
                0 => bg_mode_0(memory, vcount as u32),
                1 => bg_mode_1(memory, vcount as u32),
                2 => bg_mode_2(memory, vcount),
                3 => bg_mode_3(memory, vcount),
                4 => bg_mode_4(memory, vcount),
                5 => bg_mode_5(memory, vcount),
                _ => panic!("you can't set the bg_mode to {bg_mode}"),
            };

            let (obj_scan, obj_prio) = oam_scan(memory, vcount, dispcnt);
            let (win_scan, win_prio) = window_line();

            let combo = accumulate(
                bg_scan, obj_scan, win_scan, 
                bg_prio, obj_prio, win_prio,
            );
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

// just more convenient to mix them all together in one location
fn accumulate(
    bg: Vec<u16>, obj: Vec<u16>, win: Vec<u16>,
    bg_prio: Vec<u16>, obj_prio: Vec<u16>, win_prio: Vec<u16>,
) -> Vec<u16> {
    let mut combo = vec![0; 240];
    for i in 0..240 {
        // for now just ignore the window as im not displaying it
        let highest_priority_pixel = {
            // 0 vs 0 has 
            if bg_prio[i] < obj_prio[i] {
                bg[i]
            } else {
                obj[i]
            }
        };
        combo[i] = highest_priority_pixel;
    }
    return combo
}