use crate::memory::Memory;
use super::{convert_palette_winit, Ppu, PpuRegisters, PALETTE_BASE};

pub fn bg_mode_3(ppu: &mut Ppu, memory: &mut Memory, line: u16) {
    let mut scanline = vec![0; 240];

    let start = 0x6000000 + (line as u32 * 480); // screen width * 2
    for i in 0..240 {
        let pixel_data = memory.read_u16(start + i*2);
        let pixel = convert_palette_winit(pixel_data);
        scanline[i as usize] = pixel;
    }

    ppu.stored_screen.extend(scanline);
}
pub fn bg_mode_4(ppu: &mut Ppu, memory: &mut Memory, line: u16) {
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
    ppu.stored_screen.extend(screen);
}
pub fn bg_mode_5(ppu: &mut Ppu, memory: &mut Memory, line: u16) {
    let (width, height) = (160, 128);
    if line >= height {
        ppu.stored_screen.extend([0; 240]);
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
        let color = memory.read_u16(address + index*2);
        let pixel = convert_palette_winit(color);
        scanline[index as usize] = pixel;
    }

    ppu.stored_screen.extend(scanline);
}