use crate::{memory::Memory, SCREEN_HEIGHT, SCREEN_WIDTH};

fn convert_palette_winit(palette: u16) -> u32 {
    let (r, g, b) = (palette & 0x1F, (palette >> 5) & 0x1F, (palette >> 10) & 0x1F);
    let color = ((r as u32) << 3) << 16 | ((g as u32) << 3) << 8 | ((b as u32) << 3);
    return color 
}


enum PpuRegisters {
    Dispcnt = 0x4000000,
    //GreenSwap = 0x4000002,
    DispStat = 0x4000004,
    VCount = 0x4000006,
    BGCnt = 0x4000008,
    BgHOffset = 0x4000010,
    BgVOffset = 0x4000012,
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
                0 => bg_mode_0(ppu, memory),
                1 => bg_mode_1(ppu, memory),
                2 => bg_mode_2(ppu, memory),
                3 => bg_mode_3(ppu, memory),
                4 => bg_mode_4(ppu, memory),
                5 => bg_mode_5(ppu, memory),
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
fn bg_mode_0(ppu: &mut Ppu, memory: &mut Memory) { 
    let dispcnt = memory.read_u16(PpuRegisters::Dispcnt as u32);

    let mut priorities = Vec::new();
    let mut screen_order = Vec::new();

    // decide the order of the screens to draw
    let screens = (dispcnt >> 8) & 0xF;
    for i in 0..4 {
        let screen = (screens >> i) & 1 == 1;
        if !screen {
            continue;
        }

        let bg_cnt = memory.read_u16(PpuRegisters::BGCnt as u32 + i*2);
        let priority = bg_cnt & 0b11;

        let mut j = 0;
        while j < priorities.len() {
            if priorities[j] < priority {
                break;
            }
            j += 1;
        }

        priorities.insert(j, priority);
        screen_order.insert(j, i);
    }

    // now we go in order
    let mut overall_screen = vec![0; 240*160];

    for screen in screen_order {
        let mut inner_screens = Vec::new();

        let bg_cnt = memory.read_u16(PpuRegisters::BGCnt as u32 + screen*2);
        let size = (bg_cnt >> 14) & 0x3;

        let num_screens = match size {
            0 => 1,
            1 => 2,
            2 => 2,
            3 => 4,
            _ => unreachable!(),
        };
        for _ in 0..num_screens {
            inner_screens.push(read_screen(bg_cnt, memory));
        }

        let x_offset = memory.read_u16(PpuRegisters::BgHOffset as u32 + screen*4) as usize;
        let y_offset = memory.read_u16(PpuRegisters::BgVOffset as u32 + screen*4) as usize;

        for i in 0..140 {
            for j in 0..260 {
                let screens_y = (i + y_offset) % 256;
                let screens_x = (j + x_offset) % 256;

                let used_screen;
                match (size, i, j) {
                    (3, 256..=512, 256..=512) => used_screen = 3,
                    (3, 256..=512, _) => used_screen = 2,
                    (3, _, 256..=512) => used_screen = 1,
                    (2, 256..=512, _) => used_screen = 1,
                    (1, _, 256..=512) => used_screen = 1,
                    _ => used_screen = 0,
                }

                overall_screen[i * 240 + j] = inner_screens[used_screen][screens_y][screens_x];  
            }
        }
    }

    ppu.stored_screen = overall_screen;
    ppu.new_screen = true;
}

// index will be the number 0-3
fn read_screen(bg_cnt: u16, memory: &Memory) -> Vec<Vec<u32>> {
    let vram_base = 0x6000000;
    let mut screen = vec![vec![0; 256]; 256];

    // all the base registers and whatnot
    let screen_base_block = (bg_cnt >> 8) & 0x1F;
    let char_base_block = (bg_cnt >> 2) & 0x3;
    let bit_depth = (bg_cnt >> 7) & 1 == 1;

    let screen_address = (screen_base_block as u32 * 0x800) + vram_base;
    let charac_address = (char_base_block as u32 * 0x4000) + vram_base + 0x10000;

    // start getting all of the dots for the screen
    for drawn_index in 0..(32*32) {
        let tile_info = memory.read_u16(screen_address + drawn_index*2);
        let tile_index = tile_info & 0x1FF;

        let palette_base = 0x5000000;

        match bit_depth {
            true => {
                // 8-bit color depth
                let mut tile_address = charac_address + (tile_index as u32 * 0x40);

                for i in 0..8 {
                    for j in 0..8 {
                        let row = memory.read_u16(tile_address) as u32;
                        let palette = memory.read_u16(palette_base + row);
                        let color = convert_palette_winit(palette);

                        let screen_row = (i * 256) + (drawn_index / 32) * 256*8;
                        let screen_col = j + (drawn_index % 32) * 8;

                        screen[screen_row as usize][screen_col as usize] = color as u32;
                        tile_address += 2;
                    }
                }
            }
            false => {
                // 4-bit color depth
                let mut tile_address = charac_address + (tile_index as u32 * 0x20);
                let palette_number = (tile_info >> 12) & 0xF;
                let palette_start_entry = palette_base + (palette_number as u32 * 0x20);

                for i in 0..8 {
                    for j in 0..4 {
                        let row = memory.read_u16(tile_address) as u32;
                        let (right_pal, left_pal) = ((row >> 4) & 0xF, row & 0xF);
                        let right = memory.read_u16(palette_start_entry + right_pal * 2);
                        let left = memory.read_u16(palette_start_entry + left_pal * 2);
                        
                
                        let screen_row = ((i * 256) + (drawn_index / 32) * 256*8) as usize;
                        let screen_col = (j + (drawn_index % 32) * 8) as usize;

                        screen[screen_row % 256][(screen_col + 0) % 256] = convert_palette_winit(left);
                        screen[screen_row % 256][(screen_col + 1) % 256] = convert_palette_winit(right);

                        tile_address += 1;
                    }
                }
            }
        }
    }
    screen
}


fn bg_mode_1(ppu: &mut Ppu, memory: &mut Memory) { 

}
fn bg_mode_2(ppu: &mut Ppu, memory: &mut Memory) { 

}
fn bg_mode_3(ppu: &mut Ppu, memory: &mut Memory) {
    let total_pixels = 240*160;
    let mut screen = vec![0; total_pixels];

    let start = 0x6000000;
    let mut address = start;
    for _index in 0..total_pixels {
        let pixel_data = memory.read_u16(address);

        // all of these are values 0-31
        let (r, g, b) = (pixel_data & 0x1F, (pixel_data >> 5) & 0x1F, (pixel_data >> 10) & 0x1F);
        let (screen_r, screen_g, screen_b) = (r * 8, g * 8, b * 8);
        let final_pixel = (screen_r as u32) << 16 | (screen_b as u32) << 8 | screen_g as u32;
        screen.push(final_pixel);

        address += 2;
    }

    ppu.stored_screen = screen;
    ppu.new_screen = true;
}
fn bg_mode_4(ppu: &mut Ppu, memory: &mut Memory) {
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

        let (mut r, mut g, mut b) = ((pixel_value & 0x1F), (pixel_value >> 5) & 0x1F, (pixel_value >> 10) & 0x1F);
        r = r * 8;
        g = g * 8;
        b = b * 8;

        let pixel = (r as u32) << 16 | (g as u32) << 8 | b as u32;
        screen.push(pixel);
    }

    ppu.stored_screen = screen;
    ppu.new_screen = true;
}
fn bg_mode_5(ppu: &mut Ppu, memory: &mut Memory) {
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