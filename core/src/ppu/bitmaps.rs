use crate::memory::Memory;
use crate::memory::Memoriable;
use super::{PpuRegisters, PALETTE_BASE};

pub fn bg_mode_3(memory: &mut Box<Memory>, line: u16) -> (Vec<u16>, Vec<u16>) {
    let mut scanline = vec![0; 240];
    // let (x_offset, y_offset, _, _, _, _) = get_rotation_scaling(2, memory);

    let start = 0x6000000 + (line as u32 * 480); // screen width * 2
    for i in 0..240 {
        let pixel = memory.read_u16(start + i*2);
        scanline[i as usize] = pixel;
    }
    let priorities = {
        let priority = memory.read_u16_io(PpuRegisters::BGCnt as u32 + 0x4) & 0b11;
        vec![priority; 240]
    };
    return (scanline, priorities);
}

// TODO: this isn't always displaying text for some reason, maybe not a mode_4 issue
pub fn bg_mode_4(memory: &mut Box<Memory>, line: u16) -> (Vec<u16>, Vec<u16>) {
    let mut scanline = vec![0; 240];
    let priorities = {
        let priority = memory.read_u16_io(PpuRegisters::BGCnt as u32 + 0x4) & 0b11;
        vec![priority; 240]
    };
    let dispcnt = memory.read_u16(PpuRegisters::DispCnt as u32);
    let displayed_frame = (dispcnt >> 4) & 1 == 1;

    let mut address = match displayed_frame {
        true => 0x600A000,
        false => 0x6000000,
    };
    address += line as u32 * 240;

    for i in 0..240 {
        let palette_index = memory.read_u8(address);
        let pixel = memory.read_u16(PALETTE_BASE + (palette_index as u32 * 2));
        scanline[i] = pixel;
        address += 1;
    }
    return (scanline, priorities);
}

const MODE_5_HEIGHT: u16 = 128;
const MODE_5_WIDTH: u32 = 160;
pub fn bg_mode_5(memory: &mut Box<Memory>, line: u16) -> (Vec<u16>, Vec<u16>) {
    let mut scanline = vec![0; 240];
    let priorities = {
        let priority = memory.read_u16_io(PpuRegisters::BGCnt as u32 + 0x4) & 0b11;
        vec![priority; 240]
    };

    if line >= MODE_5_HEIGHT {
        return (scanline, priorities);
    }

    let dispcnt = memory.read_u16_io(PpuRegisters::DispCnt as u32);
    let displayed_frame = (dispcnt >> 4) & 1 == 1;

    let mut address;
    match displayed_frame {
        true => address = 0x600A000,
        false => address = 0x6000000,
    }
    address += line as u32 * 2 * MODE_5_WIDTH;

    for i in 0..MODE_5_WIDTH {
        let pixel = memory.read_u16(address + i*2);
        scanline[i as usize] = pixel;
    }
    return (scanline, priorities);
}