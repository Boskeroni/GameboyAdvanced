use crate::mem::bus::PpuInterface;
use crate::ppu::{accumulate::LineLayers, LCD_WIDTH};

const OAM: u32 = 0x7000000;
const TILE_CHAR_BLOCK: u32 = 0x6010000;

/// runs through all of the objects inside OAM and writes them (or doesn't depending
/// on priorities) to the PPU's worked_on_line. Honestly, so much stuff is happening in this
/// function that I have had to split it into so many subfunctions just to make it somewhat coherent
/// (which it really isn't).
pub fn oam_scan<P: PpuInterface>(layers: &mut LineLayers, mem: &P, vcount: u16, dispcnt: u16) {
    for obj in 0..=127 {
        // all of the attributes held by the OAMs (the 4th one isn't used yet)
        let obj_attr0 = mem.read_vram_u16(OAM + (obj * 8) + 0);
        let obj_attr1 = mem.read_vram_u16(OAM + (obj * 8) + 2);
        let obj_attr2 = mem.read_vram_u16(OAM + (obj * 8) + 4);

        if obj_attr0 == 0 && obj_attr1 == 0 && obj_attr2 == 0 {
            continue;
        }
        // these are defined here as they don't impact the reading of the tile
        // just impact if / where it is placed
        let priority = ((obj_attr2 >> 10) & 0x3) as u8;
        let obj_mode = ((obj_attr0 >> 10) & 0x3) as u8;
        let x_coord: u16 = obj_attr1 & 0x1FF;

        // this can be any amount of lines
        let new = load_obj(
            mem, 
            obj_attr0, obj_attr1, obj_attr2, 
            vcount, 
            dispcnt
        );

        for i in 0..new.len() {
            let loc = x_coord.wrapping_add(i as u16) as usize % 512;
            if loc >= LCD_WIDTH {
                continue;
            }

            let pixel = new[i];
            if pixel == 0 {
                continue;
            }
            if layers.obj_priorities[loc] <= priority {
                continue;
            }

            layers.obj[loc] = pixel;
            layers.obj_priorities[loc] = priority | (obj_mode << 2);
        }
    }
}

const SIZE_GRIDS: [[(u16, u16); 3]; 4] = [
    [(8 , 8 ), (16, 8 ), (8 , 16)],
    [(16, 16), (32, 8 ), (8 , 32)],
    [(32, 32), (32, 16), (16, 32)],
    [(64, 64), (64, 32), (32, 64)],
];

/// returns the row of pixels that would be rendered onto the vcount from the currently
/// looed at tile. This returns just an empty list if it doesn't output any pixels to the current line.
/// Once again trying to make it readable but that is quite a struggle.
fn load_obj<P: PpuInterface>(
    mem: &P, 
    obj0: u16, obj1: u16, obj2: u16, 
    vcount: u16,
    dispcnt: u16,
) -> Vec<u8> {
    let two_dimensional = (dispcnt >> 6) & 1 == 0;
    let rotation_flag = (obj0 >> 8) & 1 == 1;
    let _obj_mosaic = (obj0 >> 12) & 1 == 1;
    let is_8_bit = (obj0 >> 13) & 1 == 1;

    let mut tile_number = obj2 & 0x3FF;
    let bg_mode = dispcnt & 0b111;
    if tile_number <= 512 && bg_mode >= 3 && bg_mode <= 5 {
        return Vec::new();
    }
    if is_8_bit { // the lowest bit is ignored in 8-bit depth
        tile_number &= !(0b1);
    }

    let palette_number = ((obj2 >> 12) & 0xF) as u8;
    let y_coord = obj0 & 0xFF;

    let (width, height) = {
        let obj_shape = (obj0 >> 14) & 0x3;
        let obj_size = (obj1 >> 14) & 0x3;
        SIZE_GRIDS[obj_size as usize][obj_shape as usize]
    };

    // lets get the row of pixels that we need
    // right now just assume all of the pixels are not rotated
    let mut row_of_pixels = Vec::new();

    match rotation_flag {
        true => {
            let _double_size = (obj0 >> 9) & 1 == 1;
            let _rotation_param = (obj1 >> 9) & 0x1F;
            // println!("AFFINE sprites aint supported");
        }
        false => {
            // just not being drawn
            // weird it takes this long for it to a thing
            // and that it only really does it for when its not rotated
            let disable = (obj0 >> 9) & 1 == 1;
            if disable {
                return Vec::new();
            }

            let hor_flip = (obj1 >> 12) & 1 == 1;
            let ver_flip = (obj1 >> 13) & 1 == 1;

            // the the object would have already finished drawing by this point
            let highest_line = (y_coord + height) % 0x100;
            if highest_line < vcount {
                return Vec::new();
            }
            // the object won't need to be drawn yet
            // also takes wrapping into account
            let lowest_line = highest_line as i16 - height as i16;
            let wraps = lowest_line < 0;
            if vcount as i16 <= lowest_line {
                return Vec::new();
            }

            let unflipped_row_needed = match wraps {
                true => (0x100 - y_coord) + vcount,
                false => vcount - y_coord,
            };

            let row_needed = match ver_flip {
                true => height - unflipped_row_needed,
                false => unflipped_row_needed,
            };

            // the tile it needs to complete the row
            let tile_row = row_needed / 8;

            // this tile_wanted imagines it as an array going from 0-(however many)
            // where each 32 or whatever it is the tile below
            let tile_wanted = tile_number + 
                match two_dimensional {
                    true => 0x1F * tile_row,
                    false => tile_row * ((width / 8) - 1),
                };// if the obj is several tiles wide, then this is the earliest one
            
            match is_8_bit {
                true => {
                    // gets each tile and then each pixel in that tile
                    for i in 0..(width/8) {
                        let line_address = TILE_CHAR_BLOCK +
                            ((tile_wanted + i) as u32 * 0x40) +
                            (row_needed as u32 * 0x8);
                        
                        for pixel in 0..8 {
                            let palette_index = mem.read_vram_u8(line_address + pixel);
                            row_of_pixels.push(palette_index);
                        }
                    }
                }
                false => { // 4-bit
                    // the number of tiles 
                    for i in 0..(width/8) {
                        // its gotta be an error with this line
                        let line_address = TILE_CHAR_BLOCK +
                            ((tile_wanted + i) as u32 * 0x20) +
                            (row_needed as u32 * 0x4);

                        // since each pixel actually represents two pixels
                        for pixel in 0..4 {
                            let formatted_data = mem.read_vram_u8(line_address + pixel);

                            let left = formatted_data & 0xF;
                            match left {
                                0 => row_of_pixels.push(0),
                                _ => row_of_pixels.push((palette_number * 0x10) + left)
                            }
        
                            let right = (formatted_data >> 4) & 0xF;
                            match right {
                                0 => row_of_pixels.push(0),
                                _ => row_of_pixels.push((palette_number * 0x10) + right)
                            }
                        }
                    }
                }
            }

            if hor_flip {
                row_of_pixels.reverse();
            }
        }
    }

    return row_of_pixels;
}