use crate::{memory::{Memoriable, Memory}, ppu::{PpuRegisters, LCD_WIDTH, PALETTE_BASE}};

const OBJ_PALL: u32 = 0x5000200;

pub struct LineLayers {
    pub bgs: [[u16; LCD_WIDTH]; 4],
    pub obj: [u8; LCD_WIDTH],
    pub obj_priorities: [u8; LCD_WIDTH],
}
impl LineLayers {
    pub fn blank() -> Self {
        Self {
            // the palette entries for bg's and obj's are stored, not the pixel values
            bgs: [[0; LCD_WIDTH]; 4],
            obj: [0; LCD_WIDTH],
            // object priorites store 
            // 7654|32|10|
            // 0000|md|pr|
            // any of the 0 bits being full means its invalid and shouldnt be shown
            obj_priorities: [0xF0; LCD_WIDTH],
        }
    }
}

pub fn accumulate_and_palette(layers: &LineLayers, memory: &Box<Memory>) -> Vec<u16> {
    let bg_mode = memory.read_u16_io(PpuRegisters::DispCnt as u32) & 0x7;
    let (palette_entries, is_obj) = accumulate(layers, memory);

    let mut combo = vec![0; LCD_WIDTH];
    for i in 0..LCD_WIDTH {
        let color = match is_obj[i] {
            true => memory.read_u16(OBJ_PALL + (palette_entries[i] as u32)),
            false => {
                match bg_mode {
                    0..=2 => memory.read_u16(PALETTE_BASE + (palette_entries[i] as u32 * 2)),
                    _ => palette_entries[i],
                }
            }
        };
        combo[i] = color;
    }

    return combo;
}

// just more convenient to mix them all together in one location
fn accumulate(layers: &LineLayers, memory: &Box<Memory>) -> (Vec<u16>, Vec<bool>) {
    let mut combo = vec![0; LCD_WIDTH];
    let mut prios = vec![4; LCD_WIDTH];
    let dispcnt = memory.read_u16_io(PpuRegisters::DispCnt as u32);

    // for now just get it to be the same as before, just with all layers
    for bg in 0..=3 {
        // the display bit says its not even on
        if (dispcnt >> (8 + bg)) & 1 == 0 {
            continue;
        }

        let priority = memory.read_u16_io(PpuRegisters::BGCnt as u32 + (0x2 * bg as u32)) & 0x3;
        for i in 0..LCD_WIDTH {
            let new_pixel = layers.bgs[bg][i];
            if new_pixel == 0 { continue; }
            else if priority >= prios[i] { continue; }
            combo[i] = new_pixel;
            prios[i] = priority;
        }
    }

    // now mix that with the objs
    let mut is_obj = vec![false; LCD_WIDTH];
    for i in 0..LCD_WIDTH {
        let obj_pixel = layers.obj[i];
        let obj_prio = layers.obj_priorities[i] & 0x3;

        if obj_pixel == 0 || obj_prio == 4 { continue; }
        if obj_prio > prios[i] as u8 && combo[i] != 0 { continue; }
        combo[i] = obj_pixel as u16;
        is_obj[i] = true;
    }

    return (combo, is_obj);
}