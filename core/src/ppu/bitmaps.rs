use crate::mem::bus::PpuInterface;
use crate::ppu::LineLayers;
use crate::ppu::LCD_WIDTH;
use super::{PpuRegisters, PALETTE_BASE};

pub fn bg_mode_3<P: PpuInterface>(layers: &mut LineLayers, memory: &P, line: u16){
    let start = 0x6000000 + (line as u32 * 480); // screen width * 2
    for i in 0..LCD_WIDTH {
        let pixel = memory.read_vram_u16(start + i as u32 * 2);
        layers.bgs[2][i as usize] = pixel;
    }
}

pub fn bg_mode_4<P: PpuInterface>(layers: &mut LineLayers, memory: &P, line: u16) {
    let dispcnt = memory.read_vram_u16(PpuRegisters::DispCnt as u32);
    let displayed_frame = (dispcnt >> 4) & 1 == 1;

    let mut address = match displayed_frame {
        true => 0x600A000,
        false => 0x6000000,
    };
    address += line as u32 * LCD_WIDTH as u32;

    for i in 0..LCD_WIDTH {
        let palette_index = memory.read_vram_u8(address);
        let pixel = memory.read_vram_u16(PALETTE_BASE + (palette_index as u32 * 2));
        layers.bgs[2][i] = pixel;
        address += 1;
    }
}

const MODE_5_HEIGHT: u16 = 128;
const MODE_5_WIDTH: u32 = 160;
pub fn bg_mode_5<P: PpuInterface>(layers: &mut LineLayers, memory: &P, line: u16) {
    if line >= MODE_5_HEIGHT {
        return;
    }
    let dispcnt = memory.read_vram_u16(PpuRegisters::DispCnt as u32);
    let displayed_frame = (dispcnt >> 4) & 1 == 1;

    let mut address;
    match displayed_frame {
        true => address = 0x600A000,
        false => address = 0x6000000,
    }
    address += line as u32 * 2 * MODE_5_WIDTH;

    for i in 0..MODE_5_WIDTH {
        let pixel = memory.read_vram_u16(address + i*2);
        layers.bgs[2][i as usize] = pixel;
    }
}