use crate::memory::Memory; 
use crate::memory::Memoriable;
use super::{PpuRegisters, PALETTE_BASE, VRAM_BASE};

pub fn bg_mode_0(memory: &Box<Memory>, line: u32) -> (Vec<u16>, Vec<u16>) {
    let mut scanline = vec![0; 240];
    let mut pixel_priorities = vec![3; 240];

    let backgrounds = get_background_priorities(memory, vec![0, 1, 2, 3]);
    if backgrounds.is_empty() {
        return (scanline, pixel_priorities);
    }

    // all of the backgrounds it displays in order of priority
    for (bg, priority) in backgrounds {
        let read_line = text_mode_scanline(line, bg as u32, memory);
        for i in 0..240 {
            if scanline[i] != 0 { continue; } // already been drawn
            pixel_priorities[i] = priority as u16;
            scanline[i] = read_line[i] as u16;
        }
    }

    for i in 0..240 {
        let palette_index = scanline[i];
        let pixel = memory.read_u16(PALETTE_BASE + (palette_index as u32 * 2));
        scanline[i] = pixel;
    }
    return (scanline, pixel_priorities)
}

pub fn bg_mode_1(memory: &Box<Memory>, line: u32) -> (Vec<u16>, Vec<u16>) { 
    let mut scanline = vec![0; 240];
    let mut pixel_priorities = vec![3; 240]; // better to assume its lowest priority

    let backgrounds = get_background_priorities(memory, vec![0, 1, 2]);
    if backgrounds.is_empty() {
        return (scanline, pixel_priorities);
    }
    for (bg, priority) in backgrounds {
        // bgs 0, 1 are text
        // bg 2 is rotation
        let read_scanline = match bg {
            0|1 => text_mode_scanline(line, bg as u32, memory),
            2 => rotation_mode_scanline(line, bg as u32, memory),
            _ => unreachable!(),
        };
        for i in 0..240 {
            if scanline[i] != 0 || read_scanline[i] == 0 { continue; }
            pixel_priorities[i] = priority as u16;
            scanline[i] = read_scanline[i] as u16;
        }
    }

    for i in 0..240 {
        let palette_index = scanline[i];
        let pixel = memory.read_u16(PALETTE_BASE + (palette_index as u32 * 2));
        scanline[i] = pixel;
    }
    return (scanline, pixel_priorities);
}

pub fn bg_mode_2(_memory: &mut Memory, _line: u16) -> (Vec<u16>, Vec<u16>) { 
    todo!()
}

/// returns all the backgrounds in order of priority
/// first number is the background number, second is its priority
fn get_background_priorities(memory: &Box<Memory>, available_bgs: Vec<u32>) -> Vec<(u8, u8)> {
    let disp_cnt = memory.read_u16_io(PpuRegisters::DispCnt as u32);

    let mut priorities = vec![Vec::new(); 4];
    for bg in available_bgs {
        if (disp_cnt >> (8 + bg)) & 1 == 0 {
            continue;
        } 

        let bg_cnt = memory.read_u16_io(PpuRegisters::BGCnt as u32 + bg*2) & 0b11;
        priorities[bg_cnt as usize].push((bg as u8, bg_cnt as u8));
    }
    priorities.into_iter().flatten().collect()
}

fn rotation_mode_scanline(line: u32, bg: u32, memory: &Box<Memory>) -> Vec<u8> {
    let bg_cnt = memory.read_u16_io(PpuRegisters::BGCnt as u32 + bg * 2);
    let (width, height) = match (bg_cnt >> 14) & 0b11 {
        0 => (128, 128),
        1 => (256, 256),
        2 => (512, 512),
        3 => (1024, 1024),
        _ => unreachable!(),
    };

    let base = PpuRegisters::BgRotationBase as u32 + (bg - 2) * 0x10;
    let x0 = {
        let lower = memory.read_u16_io(base + 0x8) as i64;
        let mut higher = memory.read_u16_io(base + 0xA) as i64 & 0x0FFF;
        if (higher >> 11) & 1 == 1 {
            higher &= 0x800;
            higher = -higher;
        }
        let combine = (higher << 16 | lower) << 8;
        combine as f64 / 256.
    };
    let y0 = {
        let lower = memory.read_u16_io(base + 0xC) as i64;
        let mut higher = memory.read_u16_io(base + 0xE) as i64 & 0x0FFF;
        if (higher >> 11) & 1 == 1 {
            higher &= 0x800;
            higher = -higher
        }
        let combine = (higher << 16 | lower) << 8;
        combine as f64 / 256.
    };

    let pa = (memory.read_u16_io(base + 0x0) as f64) / 256.; // the scale factor for x
    let pb = (memory.read_u16_io(base + 0x2) as f64) / 256.; // the scale factor for y
    let pc = (memory.read_u16_io(base + 0x4) as f64) / 256.; // the shear for x
    let pd = (memory.read_u16_io(base + 0x6) as f64) / 256.; // the shear for y

    let determinant = 1. / (pa * pd - pc * pb);
    for x2 in 0..240 {
        // have to calculate both the x and the y coords
        let x1_float = determinant * (pd * (x2 as f64 - x0) - pb * (line as f64 - y0)) + x0;
        let y1_float = determinant * (pc * (x2 as f64 - x0) + pa * (line as f64 - y0)) + y0;

        let (x1, y1) = (x1_float as u32, y1_float as u32);
        if x1 >= width || y1 >= height {
            panic!("just checking if this is the wrapping this they were on about");
        }
        let row = y1 / 8;
        let col = x1 / 8;
        
        // assuming that it is 1-d mapping, not 2-d mapping
        let tile_address = VRAM_BASE + (col * 0x40) + (row * (width / 8) * 0x40);
        // let tile_index = memory.re
    }


    todo!();
}

const SCREEN_SIZE: [(u32, u32); 4] = [
    (256, 256),
    (512, 256),
    (256, 512),
    (512, 512),
];

// when i remember what this does, i will put a comment here
fn text_mode_scanline(line: u32, bg: u32, memory: &Box<Memory>) -> Vec<u8> {
    let bg_cnt = memory.read_u16_io(PpuRegisters::BGCnt as u32 + bg * 2);

    // all the variables stored within the bg_cnt register
    let char_block = (bg_cnt >> 2) & 0x3;
    let mosaic = (bg_cnt >> 6) & 1 == 1;
    let is_8_bit = (bg_cnt >> 7) & 1 == 1;
    let screen_base = (bg_cnt >> 8) & 0x1F;

    // bg modes 0-1 do not implement wrap arounds
    let wrap_around = (bg_cnt >> 13) & 1 == 1 && bg >= 2; 
    let screen_size = (bg_cnt >> 14) & 0x3;

    let (width, height) = SCREEN_SIZE[screen_size as usize];
    let sc0_address = VRAM_BASE + (screen_base as u32 * 0x800);
    let char_address = VRAM_BASE + (char_block as u32 * 0x4000);

    // all the needed knowledge of positions that we are rendering to
    // x_tile and y_tile are 0->32, offsets are 0->8
    let (mut x_tile, x_tile_offset, y_tile, y_tile_offset);
    {
        let x_offset = memory.read_u16_io(PpuRegisters::BgHOffset as u32 + (bg * 4)) as u32 & 0x1FF % width;
        let y_offset = memory.read_u16_io(PpuRegisters::BgVOffset as u32 + (bg * 4)) as u32 & 0x1FF % height;

        x_tile = x_offset / 8;
        x_tile_offset = x_offset % 8;

        y_tile = ((y_offset + line) / 8) % (height / 8);
        y_tile_offset = (y_offset + line) % 8;
    }

    // I reserve 256 as I want the list to have two tiles width
    // extra on each side, so that it accounts for scrolling
    // so 240 + (8 * 2) = 256
    let mut scanline = Vec::new();
    scanline.reserve(256);

    while scanline.len() <= 248 {
        // first figure out which screen it is rendering to
        let used_screen = match (width, height) {
            (256, 256) => 0, // it can only be SC0
            (512, 256) => (x_tile >= 32) as u32, // if its wide
            (256, 512) => (y_tile >= 32) as u32, // if its tall
            (512, 512) => (x_tile >= 32) as u32 + ((y_tile >= 32) as u32 * 2), //  if its wide or tall
            _ => unreachable!(),
        };

        // make sure it isn't just rendering nothing
        if !wrap_around {
            // we are off screen, just pad the rest with 0's and move on
            if x_tile > (width / 8) || y_tile > (height / 8) {
                for _ in scanline.len()..256 {
                    scanline.push(0);
                }
                break;
            }
        }

        // the memory address of the tile we need to get
        let tile_address = sc0_address + // baseline address all others work off
            (used_screen * 0x800) + // the screen we need to read from
            ((x_tile % 32) * 2) +  // the x_row of the tile (doubled as u16 -> u32)
            ((y_tile % 32) * 0x40); // the row being used

        let tile = memory.read_u16(tile_address);

        // all the information stored in the tile
        let tile_number = tile as u32 & 0x3FF;
        let hor_flip = (tile >> 10) & 1 == 1;
        let ver_flip = (tile >> 11) & 1 == 1;
        let palette_number = (tile >> 12) as u8 & 0xF;

        match is_8_bit {
            true => {
                let needed_row = match ver_flip {
                    true => 7 - y_tile_offset,
                    false => y_tile_offset,
                };
                let line_address = char_address +
                    (tile_number * 0x40) + 
                    (needed_row * 0x8);

                for mut pixel in 0..8 {
                    if hor_flip {
                        pixel = 7 - pixel;
                    }
                    let palette_index = memory.read_u8(line_address + pixel);
                    scanline.push(palette_index);
                }
            }
            false => {
                let needed_row = match ver_flip {
                    true => 7 - y_tile_offset,
                    false => y_tile_offset,
                };
                let line_address = char_address + 
                    (tile_number * 0x20) + 
                    (needed_row * 0x4);

                for mut pixel in 0..4 {
                    if hor_flip {
                        pixel = 3 - pixel;
                    }
                    let formatted_data = memory.read_u8(line_address + pixel);

                    let left;
                    match hor_flip {
                        true => left = (formatted_data >> 4) & 0xF,
                        false => left = formatted_data & 0xF,
                    }
                    match left {
                        0 => scanline.push(0),
                        _ => scanline.push((palette_number * 0x10) + left)
                    }

                    let right;
                    match hor_flip {
                        true => right = formatted_data & 0xF,
                        false => right = (formatted_data >> 4) & 0xF,
                    }
                    match right {
                        0 => scanline.push(0),
                        _ => scanline.push((palette_number * 0x10) + right)
                    }
                }
            }
        }

        x_tile += 1;
        x_tile %= width / 8;
    }

    scanline = scanline[(x_tile_offset as usize)..(x_tile_offset as usize + 240)].to_vec();
    return scanline
}