use pixels::wgpu::core::resource::BufferAccessResult;

use crate::{memory::Memory, SCREEN_HEIGHT, SCREEN_WIDTH};

enum PpuRegisters {
    Dispcnt = 0x4000000,
    //GreenSwap = 0x4000002,
    DispStat = 0x4000004,
    VCount = 0x4000006,
}

pub struct Ppu {
    pub new_screen: bool,
    elapsed_time: usize, // represents the number of dots elapsed
    pub stored_screen: Vec<u32>,
}
impl Ppu {
    pub fn new() -> Self {
        Self { 
            new_screen: false,
            elapsed_time: 0,
            stored_screen: Vec::new(),
        }
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
    let mut dispstat = memory.read_u16(PpuRegisters::DispStat as u32);
    let mut vcount = memory.read_u16(PpuRegisters::VCount as u32) as usize;

    let new_line = ppu.elapsed_time / (SCREEN_WIDTH + 68) != vcount;
    if new_line {
        vcount += 1;

        // new frame
        if vcount >= (SCREEN_HEIGHT + 68) {
            // right now we are just displaying at end of every frame
            let bg_mode = dispcnt & 0b111;
            match bg_mode {
                0 => mode_0_display(ppu, memory),
                1 => mode_1_display(ppu, memory),
                2 => mode_2_display(ppu, memory),
                3 => mode_3_display(ppu, memory),
                4 => mode_4_display(ppu, memory),
                5 => mode_5_display(ppu, memory),
                _ => panic!("you can't set the bg_mode to {bg_mode}"),
            }
            vcount = 0;
            ppu.elapsed_time = 0;
        }
        memory.write_io(PpuRegisters::VCount as u32, vcount as u16);
        
        let vcount_lyc = (dispstat >> 8) & 0xFF;
        let vcounter_match = vcount == vcount_lyc as usize;
        match vcounter_match {
            true => dispstat |= 1<<2,
            false => dispstat &= !(1<<2),
        }        
    }

    update_registers(ppu, memory, dispstat);
}

fn update_registers(ppu: &mut Ppu, memory: &mut Memory, mut dispstat: u16) {
    // work in progress
    ppu.elapsed_time += 20;
    
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


    let mut ie = memory.read_u16(0x4000202);
    ie &= !0b111;
    ie |= dispstat & 0b111;

    memory.write_io(0x4000202, ie);
    memory.write_io(PpuRegisters::DispStat as u32, dispstat);
}


// all of the different displaying functions
fn mode_0_display(ppu: &mut Ppu, memory: &mut Memory) { 

}
fn mode_1_display(ppu: &mut Ppu, memory: &mut Memory) { 

}
fn mode_2_display(ppu: &mut Ppu, memory: &mut Memory) { 

}
fn mode_3_display(ppu: &mut Ppu, memory: &mut Memory) {
    let total_pixels = 240*160;
    let mut screen = vec![0; total_pixels];

    let start = 0x6000000;
    let mut address = start;
    for _index in 0..total_pixels {
        let pixel_data = memory.read_u16(address);

        // all of these are values 0-31
        let (r, g, b) = (pixel_data & 0x1F, (pixel_data >> 5) & 0x1F, (pixel_data >> 10) & 0x1F);
        let (screen_r, screen_g, screen_b) = ((r / 31) * 0xFF, (g / 31) * 0xFF, (b / 31) * 0xFF);
        let final_pixel = (screen_r as u32) << 16 | (screen_b as u32) << 8 | screen_g as u32;
        screen.push(final_pixel);

        address += 2;
    }

    ppu.stored_screen = screen;
    ppu.new_screen = true;
}
fn mode_4_display(ppu: &mut Ppu, memory: &mut Memory) {
    let dispcnt = memory.read_u16(PpuRegisters::Dispcnt as u32);
    let displayed_frame = (dispcnt >> 4) & 1 == 1;

    let base;
    let palette_base = 0x5000000;
    match displayed_frame {
        true => base = 0x600A000,
        false => base = 0x6000000,
    }

    let mut screen = Vec::new();
    let total_pixels = 240*160;
    for index in 0..total_pixels {
        let palette_index = memory.read_u8(base + index);
        let pixel_value = memory.read_u16(palette_base + (palette_index as u32 * 2));
        screen.push(pixel_value as u32);
    }

    ppu.stored_screen = screen;
    ppu.new_screen = true;
}
fn mode_5_display(ppu: &mut Ppu, memory: &mut Memory) {
    let width = 160;
    let height = 128;

    let dispcnt = memory.read_u16(PpuRegisters::Dispcnt as u32);
    let displayed_frame = (dispcnt >> 4) & 1 == 1;

    let base;
    match displayed_frame {
        true => base = 0x600A000,
        false => base = 0x6000000,
    }

    let mut screen = Vec::new();
    let total_pixels = width * height;
    for index in 0..total_pixels {
        let color = memory.read_u16(base + index*2);

        // all of these are values 0-31
        let (r, g, b) = (color & 0x1F, (color >> 5) & 0x1F, (color >> 10) & 0x1F);
        let (screen_r, screen_g, screen_b) = ((r / 31) * 0xFF, (g / 31) * 0xFF, (b / 31) * 0xFF);
        let final_pixel = (screen_r as u32) << 16 | (screen_b as u32) << 8 | screen_g as u32;
        screen.push(final_pixel);

        // accounting for the fact its only 160 pixels wide
        // the screen expects it to be 240
        if index % width == 0{
            screen.extend(vec![0_u32; 80]);
        }
    }

    ppu.stored_screen = screen;
    ppu.new_screen = true;
}