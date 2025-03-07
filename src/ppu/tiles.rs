use crate::memory::Memory;

use super::{convert_palette_winit, Ppu, PpuRegisters, PALETTE_BASE, VRAM_BASE};

pub fn bg_mode_0(ppu: &mut Ppu, memory: &mut Memory, line: u32) {
    let dispcnt = memory.read_u16(PpuRegisters::DispStat as u32);

    // the index of the background and its priority.
    // sorted lowest to highest priority.
    let mut backgrounds = Vec::new();
    let mut priorities = Vec::new();
    for i in 0..=3 {
        if dispcnt >> (8 + i) & 1 == 1 {
            continue;
        }

        let bg_cnt = memory.read_u16(PpuRegisters::BGCnt as u32 + i*2);
        let priority = bg_cnt & 0b11;

        // cheeky little insertions
        let mut curr = 0;
        while curr < backgrounds.len()  {
            if backgrounds[curr] <= priority as u32  {
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
    for bg in backgrounds {
        let line = read_scanline(line, bg, memory);
        for i in 0..240 {
            // something has already been displayed to the scanline
            if scanline[i] != 0 { continue; }
            scanline[i] = line[i];
        }
    }
    ppu.stored_screen.extend(scanline);
}
fn read_scanline(line: u32, bg: u32, memory: &mut Memory) -> Vec<u32> {
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
    let (mut x_tile, x_tile_offset, tile_row, tile_row_offset);
    {
        let x_offset = memory.read_u16(PpuRegisters::BgHOffset as u32 + (bg * 4)) as u32;
        let y_offset = memory.read_u16(PpuRegisters::BgVOffset as u32 + (bg * 4)) as u32;

        x_tile = x_offset / 8;
        x_tile_offset = x_offset % 8;

        tile_row = (y_offset + line) / 8;
        tile_row_offset = (y_offset + line) % 8;
    }

    // I reserve 248 as it is the minimum amount that we will store
    // on any given run. Could do more, but the performance gain would
    // be so incredibly minimal that it would be counter-acted by the
    // times it doesnt need it
    let mut scanline = Vec::<u32>::new();
    scanline.reserve(248);

    while scanline.len() <= 248 {
        // first figure out which screen it is rendering to
        let used_screen = match (width, height) {
            (256, 256) => 0,
            (512, 256) => (x_tile >= 32) as u32,
            (256, 512) => (tile_row >= 32) as u32,
            (512, 512) => (x_tile >= 32) as u32 + ((tile_row >= 32) as u32) * 2,
            _ => unreachable!(),
        };

        // the memory address of the tile we need to get
        let tile_address = sc0_address + // baseline address all others work off
            (used_screen * 0x800) + // the screen we need to read from
            ((x_tile % 32) * 2) +  // the x_row of the tile (doubled as u16 -> u32)
            ((tile_row % 32) * 0x40); // the row being used
        let tile = memory.read_u16(tile_address);

        // all the information stored in the tile
        let tile_number = tile as u32 & 0x1FF;
        let hor_flip = (tile >> 10) & 1 == 1;
        let ver_flip = (tile >> 11) & 1 == 1;
        let palette_number = (tile >> 12) as u32 & 0xF;

        match is_8_bit {
            true => {
                let needed_row = match ver_flip {
                    true => 7 - tile_row_offset,
                    false => tile_row_offset,
                };
                let line_address = char_address + // base? im not sure if its this or 0x60... flat
                    (tile_number * 0x40) + // 
                    (needed_row * 0x8);

                for mut pixel in 0..8 {
                    if hor_flip {
                        pixel = 7 - pixel;
                    }

                    let palette_index = memory.read_u8(line_address + pixel);
                    let palette = memory.read_u16(PALETTE_BASE + (palette_index as u32 * 2));
                    let screen_value = convert_palette_winit(palette);
                    scanline.push(screen_value);
                }
            }
            false => {
                let needed_row = match ver_flip {
                    true => 7 - tile_row_offset,
                    false => tile_row_offset,
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
                    let left_palette = memory.read_u16(PALETTE_BASE + (palette_number * 0x20) + (left as u32 * 2));
                    let left_screen_value = convert_palette_winit(left_palette);
                    scanline.push(left_screen_value);

                    let right = (formatted_data >> 4) & 0xF;
                    let right_palette = memory.read_u16(PALETTE_BASE + (palette_number * 0x20) + (right as u32 * 2));
                    let right_screen_value = convert_palette_winit(right_palette);
                    scanline.push(right_screen_value);
                }
            }
        }

        x_tile += 1;
        x_tile %= 64;
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
