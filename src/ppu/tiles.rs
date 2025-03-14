use crate::memory::Memory;

use super::{convert_palette_winit, Ppu, PpuRegisters, PALETTE_BASE, VRAM_BASE};

pub fn bg_mode_0(ppu: &mut Ppu, memory: &mut Memory, line: u32) {
    let dispcnt = memory.read_u16(PpuRegisters::DispCnt as u32);

    // the index of the background and its priority.
    // highest priority to lowest priority
    // 3 = lowest, 0 = highest
    let mut backgrounds = Vec::new();
    let mut priorities = Vec::new();
    for i in 0..=3 {
        // the background is turned off
        if dispcnt >> (8 + i) & 1 == 0 {
            continue;
        }

        let bg_cnt = memory.read_u16(PpuRegisters::BGCnt as u32 + i*2);
        let priority = bg_cnt & 0b11;

        // cheeky little insertions
        let mut curr = 0;
        while curr < priorities.len()  {
            if priorities[curr] > priority  {
                break;
            }
            curr += 1;
        }
        backgrounds.insert(curr, i);
        priorities.insert(curr, priority);
    }

    // can't display anything to any background
    if backgrounds.is_empty() {
        return;
    }

    // now have a list of the order of the backgrounds
    let mut scanline = vec![0; 240];
    let mut pixel_priorities = vec![0; 240];
    for j in 0..backgrounds.len() {
        if j != 0 { continue; }
        let bg = backgrounds[j];
        let priority = priorities[j];

        let read_line = read_scanline(line, bg, memory);
        for i in 0..240 {
            // something has already been displayed to the scanline
            if scanline[i] != 0 { continue; }
            pixel_priorities[i] = priority;
            scanline[i] = read_line[i];
        }
    }
    
    ppu.pixel_priorities = pixel_priorities;
    ppu.worked_on_line = scanline.iter().map(|&palette_index| 
        convert_palette_winit(memory.read_u16(PALETTE_BASE + (palette_index as u32 * 2)))
    ).collect();
}
fn read_scanline(line: u32, bg: u32, memory: &mut Memory) -> Vec<u8> {
    let bg_cnt = memory.read_u16(PpuRegisters::BGCnt as u32 + bg * 2);

    // all the variables stored within the bg_cnt register
    // (not all of their functionality have been implemented yet)
    let char_block = (bg_cnt >> 2) & 0x3;
    let mosaic = (bg_cnt >> 6) & 1 == 1;
    let is_8_bit = (bg_cnt >> 7) & 1 == 1;
    let screen_base = (bg_cnt >> 8) & 0x1F;
    let wrap_around = (bg_cnt >> 13) & 1 == 1 && bg != 0 && bg != 1;
    let screen_size = (bg_cnt >> 14) & 0x3;

    let (width, height) = match screen_size {
        0 => (256, 256),
        1 => (512, 256),
        2 => (256, 512),
        3 => (512, 512),
        _ => unreachable!(),
    };

    let sc0_address = VRAM_BASE + (screen_base as u32 * 0x800);
    let char_address = VRAM_BASE + (char_block as u32 * 0x4000);

    // all the needed knowledge of positions that we are rendering to
    // x_tile and y_tile are 0->32, offsets are 0->8
    let (mut x_tile, x_tile_offset, y_tile, y_tile_offset);
    {
        let x_offset = memory.read_u16(PpuRegisters::BgHOffset as u32 + (bg * 4)) as u32 & 0x1FF;
        let y_offset = memory.read_u16(PpuRegisters::BgVOffset as u32 + (bg * 4)) as u32 & 0x1FF;

        x_tile = x_offset / 8;
        x_tile_offset = x_offset % 8;

        match wrap_around {
            true => y_tile = ((y_offset + line) / 8) % (height / 8),
            false => y_tile = (y_offset + line) / 8,
        }
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
            (512, 512) => (x_tile >= 32) as u32 + ((y_tile >= 32) as u32) * 2, //  if its wide or tall
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
        let tile_number = tile as u32 & 0x1FF;
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
                        pixel = 7 - pixel;
                    }
                    let formatted_data = memory.read_u8(line_address + pixel);

                    let left = formatted_data & 0xF;
                    let left_palette_index = (palette_number * 0x10) + left;
                    scanline.push(left_palette_index);

                    let right = (formatted_data >> 4) & 0xF;
                    let right_palette_index = (palette_number * 0x10) + right;
                    scanline.push(right_palette_index);
                }
            }
        }

        x_tile += 1;
        if wrap_around {
            x_tile %= 64;
        }
    }

    scanline = scanline[(x_tile_offset as usize)..(x_tile_offset as usize + 240)].to_vec();
    return scanline
}
pub fn bg_mode_1(ppu: &mut Ppu, memory: &mut Memory, line: u16) { 
    todo!()
}
pub fn bg_mode_2(ppu: &mut Ppu, memory: &mut Memory, line: u16) { 
    todo!()
}
