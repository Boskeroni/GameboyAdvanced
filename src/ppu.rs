use crate::{memory::Memory, SCREEN_HEIGHT, SCREEN_WIDTH};

const VRAM_BASE: u32 = 0x6000000;
const VRAM_TILE_BASE: u32 = 0x6010000;
const PALETTE_BASE: u32 = 0x5000000;

fn convert_palette_winit(palette: u16) -> u32 {
    let (r, g, b) = (palette & 0x1F, (palette >> 5) & 0x1F, (palette >> 10) & 0x1F);
    let color = ((r as u32) << 3) << 16 | ((g as u32) << 3) << 8 | ((b as u32) << 3);
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
    Mosaic = 0x400004C,
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
    let mut dispstat = memory.read_u16(PpuRegisters::DispStat as u32);
    let mut vcount = memory.read_u16(PpuRegisters::VCount as u32);

    let new_line = ppu.elapsed_time / (SCREEN_WIDTH + 68) != vcount as usize;
    if new_line {
        vcount += 1;

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

        // new frame
        if vcount as usize >= (SCREEN_HEIGHT + 68) {
            // right now we are just displaying at end of every frame
            vcount = 0;
        }
        memory.write_io(PpuRegisters::VCount as u32, vcount as u16);
        
        let vcount_lyc = (dispstat >> 8) & 0xFF;
        let vcounter_match = vcount == vcount_lyc;
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

fn bg_mode_0(ppu: &mut Ppu, memory: &mut Memory, line: u32) {
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

    if backgrounds.is_empty() {
        return;
    }

    // now have a list of the order of the backgrounds
    let mut scanline = vec![0; 240];
    for bg in backgrounds {
        let line = read_scanline(line, bg, memory);
        for i in 0..240 {
            if scanline[i] != 0 { continue; }
            scanline[i] = line[i];
        }
    }
    ppu.stored_screen.extend(scanline);
}
fn read_scanline(line: u32, bg: u32, memory: &mut Memory) -> Vec<u32> {
    let bg_cnt = memory.read_u16(PpuRegisters::BGCnt as u32 + bg * 2);

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

    let (mut x_tile, x_tile_offset, tile_row, tile_row_offset);
    {
        let x_offset = memory.read_u16(PpuRegisters::BgHOffset as u32 + (bg * 4)) as u32;
        let y_offset = memory.read_u16(PpuRegisters::BgVOffset as u32 + (bg * 4)) as u32;

        x_tile = x_offset / 8;
        x_tile_offset = x_offset % 8;

        tile_row = (y_offset + line) / 8;
        tile_row_offset = (y_offset + line) % 8;
    }

    let mut scanline = Vec::<u32>::new();
    scanline.reserve(240);

    while scanline.len() <= 248 {
        // first figure out where the tile we need is
        let used_screen = match (width, height) {
            (256, 256) => 0,
            (512, 256) => (x_tile >= 32) as u32,
            (256, 512) => (tile_row >= 32) as u32,
            (512, 512) => (x_tile >= 32) as u32 + ((tile_row >= 32) as u32) * 2,
            _ => unreachable!(),
        };

        let tile_address = sc0_address + ((x_tile % 32) * 2) + ((tile_row % 32) * 0x40) + (used_screen * 0x800);
        let tile = memory.read_u16(tile_address);

        let tile_number = tile as u32 & 0x1FF;
        let hor_flip = (tile >> 10) & 1 == 1;
        let ver_flip = (tile >> 11) & 1 == 1;

        match is_8_bit {
            true => {
                let needed_row = match ver_flip {
                    true => 7 - tile_row_offset,
                    false => tile_row_offset,
                };
                let line_address = char_address + (tile_number * 0x40) + (needed_row * 0x8);

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
                let palette_number = (tile >> 12) as u32 & 0xF;
                let needed_row = match ver_flip {
                    true => 7 - tile_row_offset,
                    false => tile_row_offset,
                };
                let line_address = char_address + (tile_number * 0x20) + (needed_row * 0x4);

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


fn bg_mode_1(ppu: &mut Ppu, memory: &mut Memory, line: u16) { 
    todo!();
}
fn bg_mode_2(ppu: &mut Ppu, memory: &mut Memory, line: u16) { 
    todo!();
}
fn bg_mode_3(ppu: &mut Ppu, memory: &mut Memory, line: u16) {
    let mut scanline = vec![0; 240];

    let start = 0x6000000 + (line as u32 * 480); // screen width * 2
    let mut address = start;
    for _ in 0..240 {
        let pixel_data = memory.read_u16(address);
        let pixel = convert_palette_winit(pixel_data);
        scanline.push(pixel);
        address += 2;
    }

    ppu.stored_screen.extend(scanline);
}
fn bg_mode_4(ppu: &mut Ppu, memory: &mut Memory, line: u16) {
    let dispcnt = memory.read_u16(PpuRegisters::Dispcnt as u32);
    let displayed_frame = (dispcnt >> 4) & 1 == 1;

    let mut address;
    match displayed_frame {
        true => address = 0x600A000,
        false => address = 0x6000000,
    }
    address += line as u32 * 240;

    let mut screen = Vec::new();
    for _ in 0..240 {
        let palette_index = memory.read_u8(address);
        let pixel_value = memory.read_u16(PALETTE_BASE + (palette_index as u32 * 2));
        let pixel = convert_palette_winit(pixel_value);
        screen.push(pixel);
        address += 1;
    }

    ppu.stored_screen = screen;
}
fn bg_mode_5(ppu: &mut Ppu, memory: &mut Memory, line: u16) {
    let width = 160;
    let height = 128;

    if line >= height {
        return;
    }

    let dispcnt = memory.read_u16(PpuRegisters::Dispcnt as u32);
    let displayed_frame = (dispcnt >> 4) & 1 == 1;

    let mut address;
    match displayed_frame {
        true => address = 0x600A000,
        false => address = 0x6000000,
    }
    address += line as u32 * 2 * width;

    let mut scanline = vec![0; 240];
    for index in 0..width {
        let color = memory.read_u16(address);
        let pixel = convert_palette_winit(color);
        scanline[index as usize] = pixel;
    }

    ppu.stored_screen.extend(scanline);
}