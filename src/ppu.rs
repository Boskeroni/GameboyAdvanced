use crate::{memory::Memory, SCREEN_HEIGHT, SCREEN_WIDTH};

enum PpuRegisters {
    Dispcnt = 0x4000000,
    GreenSwap = 0x4000002,
    LcdStatus = 0x4000004,
    VCount = 0x4000006,
}

pub fn update_ppu(memory: &mut Memory) -> Option<Vec<u16>> {
    let dispcnt = memory.read_u16(PpuRegisters::Dispcnt as u32);

    let bg_mode = dispcnt & 0b111;
    

    match bg_mode {
        0 => return None,
        1 => return None,
        2 => return None,
        3 => return mode_3(memory),
        4 => return mode_4(memory, dispcnt),
        5 => return None,
        _ => panic!("invalid ppu mode"),
    }
}

fn mode_3(memory: &mut Memory) -> Option<Vec<u16>> {
    return None;
}

fn mode_4(memory: &mut Memory, dispcnt: u16) -> Option<Vec<u16>> {
    let palette_base = 0x5000000;
    return None;
}