use crate::{memory::Memory, SCREEN_HEIGHT, SCREEN_WIDTH};

enum PpuRegisters {
    Dispcnt = 0x4000000,
    GreenSwap = 0x4000002,
    LcdStatus = 0x4000004,
    VCount = 0x4000006,
}

pub fn update_ppu(memory: &mut Memory) -> Vec<u32> {
    let dispcnt = memory.read_u16(PpuRegisters::Dispcnt as u32);
    let ppu_mode = dispcnt & 0b111;

    let buffer = ppu_mode_4(memory);
    return buffer;

    // match ppu_mode {
    //     0b000 => ppu_mode_0(),
    //     0b001 => ppu_mode_1(),
    //     0b010 => ppu_mode_2(),
    //     0b011 => ppu_mode_3(),
    //     0b100 => ppu_mode_4(memory),
    //     0b101 => ppu_mode_5(),
    //     _ => panic!("ppu display mode is invalid"),
    // }
}

fn ppu_mode_0() {}
fn ppu_mode_1() {}
fn ppu_mode_2() {}
fn ppu_mode_3() {}

// im starting with mode 4 cause thats what Arm.gba needs first
fn ppu_mode_4(memory: &mut Memory) -> Vec<u32> {
    let palette_base_address = 0x5000000;
    let background_base_address = 0x6000000;

    let mut buffer = vec![0; SCREEN_WIDTH*SCREEN_HEIGHT];

    for i in 0..(SCREEN_HEIGHT*SCREEN_WIDTH) {
        let entry_address = background_base_address + i;
        let palette_index = memory.read_u8(entry_address as u32) as u32;
        let color = memory.read_u16((palette_index * 2) + palette_base_address);

        buffer[i as usize] = color as u32;
    }
    return buffer;
    // now what the fuck do I do with the buffer

}
fn ppu_mode_5() {}
