use crate::{memory::Memory, SCREEN_HEIGHT, SCREEN_WIDTH};

enum PpuRegisters {
    Dispcnt = 0x4000000,
    //GreenSwap = 0x4000002,
    DispStat = 0x4000004,
    VCount = 0x4000006,
}

pub struct PpuState {
    elapsed_time: usize, // represents the number of dots elapsed
}
impl PpuState {
    pub fn new() -> Self {
        Self {
            elapsed_time: 0,
        }
    }
}
const DOTS_PER_FRAME: usize = (SCREEN_WIDTH + 68) * (SCREEN_HEIGHT + 68);
// so far this will just update the registers
// cba making it work just yet (soon though)
pub fn update_ppu(ppu: &mut PpuState, memory: &mut Memory) {
    let dispcnt = memory.read_u16(PpuRegisters::Dispcnt as u32);
    let bg_mode = dispcnt & 0b111;

    match bg_mode {
        0 => {}
        1 => {}
        2 => {}
        3 => {}
        4 => {}
        5 => {}
        _ => panic!("you can't set the bg_mode to {bg_mode}"),
    }

    let mut dispstat = memory.read_u16(PpuRegisters::DispStat as u32);

    // TODO: eventually make it so it matches the CPU timings
    ppu.elapsed_time += 1;

    // V-Blank flag
    if ppu.elapsed_time >= DOTS_PER_FRAME {
        ppu.elapsed_time = 0;
        dispstat &= !(1<<0);
    }
    // H-blank flag
    if ppu.elapsed_time % (SCREEN_WIDTH + 68) >= SCREEN_WIDTH {
        dispstat |= 1<<1;
    } else {
        dispstat &= !(1<<1);
    }

    // V-Count flag
    let mut vcount = memory.read_u16(PpuRegisters::VCount as u32);
    let vcount_lyc = (dispstat >> 8) & 0xFF;

    if (vcount as usize) != ppu.elapsed_time / (SCREEN_WIDTH + 68) {
        vcount += 1;
        if vcount as usize >= (SCREEN_HEIGHT + 68) {
            vcount = 0;
        }
        // we are on the next line now
        memory.write_io(PpuRegisters::VCount as u32, vcount);
        if vcount == vcount_lyc {
            dispstat |= 1<<2;
        } else {
            dispstat &= !(1<<2);
        }
    }  
    memory.write_io(PpuRegisters::DispStat as u32, dispstat);

}