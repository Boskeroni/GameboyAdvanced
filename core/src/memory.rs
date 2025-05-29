// this has been acquired legally
const BIOS: &[u8; 0x4000] = include_bytes!("../bios.bin");

/// output =>
/// 0bBBBBBBBBAAAAAAAA
#[inline]
fn lil_end_combine_u16(a: u8, b: u8) -> u16 {
    return ((b as u16) << 8) + a as u16
}

/// input => 0bBBBBBBBBBAAAAAAAA
/// output => (0bAAAAAAAA, 0bBBBBBBBB)
#[inline]
fn lil_end_split_u16(a: u16) -> (u8, u8) {
    return (a as u8, (a >> 8) as u8)
}

/// output => 
/// 0bDDDDDDDDCCCCCCCCBBBBBBBBAAAAAAAA
#[inline]
fn lil_end_combine_u32(a: u8, b: u8, c: u8, d: u8) -> u32 {
    let (a, b, c, d) = (a as u32, b as u32, c as u32, d as u32);
    return (d<<24) | (c<<16) | (b<<8) | (a);
}

/// input => 0bDDDDDDDDCCCCCCCCBBBBBBBBAAAAAAAA
/// output => (0bAAAAAAAA, 0bBBBBBBBB, 0bCCCCCCCC, 0bDDDDDDDD)
#[inline]
fn lil_end_split_u32(a: u32) -> (u8, u8, u8, u8) {
    return (a as u8, (a >> 8) as u8, (a >> 16) as u8, (a >> 24) as u8)
}

#[inline]
fn split_memory_address(address: u32) -> (u32, usize) {
    ((address >> 24) & 0xF, (address & 0xFFFFF) as usize)
}

const DMA_SAD: u32 = 0x40000B0;
const DMA_DAD: u32 = 0x40000B4;
const DMA_AMOUNT: u32 = 0x40000B8;
const DMA_CONTROL: u32 = 0x40000BA;

const EWRAM_LENGTH: usize = 0x40000;
const IWRAM_LENGTH: usize = 0x8000;
const VRAM_LENGTH: usize = 0x18000;
const IO_REG_LENGTH: usize = 0x400;
const OBJ_PALL_LENGTH: usize = 0x400;
const OAM_LENGTH: usize = 0x400;
// this could be wrong, just the max value is safer
const SRAM_MAX_LENGTH: usize = 0x10000;

pub struct Memory {
    ewram: [u8; EWRAM_LENGTH], // WRAM - On-board Work RAM
    iwram: [u8; IWRAM_LENGTH],  // WRAM - On-chip Work RAM
    vram: [u8; VRAM_LENGTH], // 96 KB - 16 bit bus
    io_reg: [u8; IO_REG_LENGTH],
    obj_pall: [u8; OBJ_PALL_LENGTH],
    oam: [u8; OAM_LENGTH],
    gp_rom: Vec<u8>,
    sram: [u8; SRAM_MAX_LENGTH],

    timer_resets: [u16; 4],
    dma_completions: [u32; 4],
}
impl Memory {
    pub fn read_u8(&self, address: u32) -> u8 {
        let (upp_add, low_add) = split_memory_address(address);

        match upp_add {
            0x0 => BIOS[low_add % BIOS.len()],
            0x2 => self.ewram[low_add % EWRAM_LENGTH],
            0x3 => self.iwram[low_add % IWRAM_LENGTH],
            0x4 => self.io_reg[low_add % IO_REG_LENGTH],
            0x5 => self.obj_pall[low_add % OBJ_PALL_LENGTH],
            0x6 => {
                // 64k-32k (then the 32k is mirrored again) (then everything is mirrored again)
                let base = low_add % 0x20000;
                if base >= 0x10000 {
                    return self.vram[0x10000 + (base % 0x8000)]
                }
                self.vram[base]
            }
            0x7 => self.oam[low_add % OAM_LENGTH],
            0xE => self.sram[low_add % SRAM_MAX_LENGTH],
            _ => { // assuming this is just ROM
                // deals with the mirrors
                // this will be more important once I start dealing with timings
                // let unmirrored_address = (address % 0x2000000) + 0x8000000;

                // this is just cause some games store this in prefetch but don't use it
                if low_add >= self.gp_rom.len() {
                    return 0x00;
                }
                self.gp_rom[low_add]
            },
        }
    }

    pub fn read_u16(&self, address: u32) -> u16 {
        let base_address = address & !(0b1);

        lil_end_combine_u16(
            self.read_u8(base_address + 0), 
            self.read_u8(base_address + 1)
        )
    }

    pub fn read_u32(&self, address: u32) -> u32 {
        let base_address = address & !(0b11);

        lil_end_combine_u32(
            self.read_u8(base_address + 0), 
            self.read_u8(base_address + 1), 
            self.read_u8(base_address + 2), 
            self.read_u8(base_address + 3),
        )
    }

    fn checked_write_u8(&mut self, address: u32, data: u8, is_8_bit: bool) {
        if address == 0x4000202 || address == 0x4000203 {
            self.io_reg[address as usize - 0x4000000] &= !data;
            return;
        }

        // writing to a timer register
        if address >= 0x4000100 && address <= 0x400010E && address % 4 < 2 {
            // this rounds down anyways which is good
            let timer_specified = (address - 0x4000100) / 4;
            let write_upper_byte = address & 1 == 1;
            let data = data as u16;

            let timer_reset = &mut self.timer_resets[timer_specified as usize];
            match write_upper_byte {
                true => {*timer_reset &= 0x00FF; *timer_reset |= data << 8},
                false => {*timer_reset &= 0xFF00; *timer_reset |= data},
            }
            return;
        }
        
        let (upp_add, low_add) = split_memory_address(address);
        
        // why do the video memory buffers not allow 8-bit writes??
        // no clue but it does
        if is_in_video_memory(upp_add) && is_8_bit {
            // no chance of a write happening
            if upp_add == 7 {
                return;
            }

            // why is this a thing
            let bg = self.io_reg[0] & 0x7;
            let bitmap = bg >= 4;
            let mut write_both = false;
            if upp_add == 0x6 {
                match bitmap {
                    true => write_both |= low_add <= 0xFFFF,
                    false => write_both |= low_add <= 0x13FFF,
                }
            }
            // pallete
            write_both |= upp_add == 0x5;
            
            if !write_both {
                return;
            }

            // just mirrors it up and down
            // since should be recursive as is_8_bit will be set to false
            self.write_u16(address & !1, (data as u16) * 0x101);
            return;
        }

        match upp_add {
            0x0 => panic!("cannot make a write to the BIOS"),
            0x2 => self.ewram[low_add % EWRAM_LENGTH] = data,
            0x3 => self.iwram[low_add % IWRAM_LENGTH] = data,
            0x4 => self.io_reg[low_add % IO_REG_LENGTH] = data,
            0x5 => self.obj_pall[low_add % OBJ_PALL_LENGTH] = data,
            0x6 => {
                self.vram[low_add % VRAM_LENGTH] = data
            }
            0x7 => self.oam[low_add % OAM_LENGTH] = data,
            0xE => self.sram[low_add % SRAM_MAX_LENGTH] = data,
            _ => {},
        };
    }

    pub fn write_u8(&mut self, address: u32, data: u8) {
        self.checked_write_u8(address, data, true);
    }

    pub fn write_u16(&mut self, address: u32, data: u16) {
        let split = lil_end_split_u16(data);
        let address = address & !(0b1);

        self.checked_write_u8(address + 0, split.0, false);
        self.checked_write_u8(address + 1, split.1, false);
    }

    pub fn write_u32(&mut self, address: u32, data: u32) {
        let split = lil_end_split_u32(data);
        let address = address & !(0b11);

        self.checked_write_u8(address + 0, split.0, false);
        self.checked_write_u8(address + 1, split.1, false);
        self.checked_write_u8(address + 2, split.2, false);
        self.checked_write_u8(address + 3, split.3, false);
    }

    /// avoids all of the checks, used just by the other sub-systems
    pub fn write_io(&mut self, address: u32, data: u16) {
        let address = (address as usize - 0x4000000) & !(0b1);
        let split = lil_end_split_u16(data);

        self.io_reg[address + 0] = split.0;
        self.io_reg[address + 1] = split.1;
    }
}

#[inline]
fn is_in_video_memory(upp_add: u32) -> bool {
    return (upp_add == 0x5) | (upp_add == 0x6) | (upp_add == 0x7);
}

// since DMA takes several cycles, its best to just have it be its own thing
pub fn dma_tick(mem: &mut Memory) -> bool {
    let mut dma_transfer = None;
    for i in 0..=3 {
        let cnt = mem.read_u16(DMA_CONTROL + i*0xC);
        let is_on = (cnt >> 15) & 1 == 1;

        // highest priority goes 0 -> 3
        if is_on {
            dma_transfer = Some((i, cnt));
            break;
        }
    }

    // no dma transfer active rn
    if let None = dma_transfer {
        return false;
    }
    let (i, cnt) = dma_transfer.unwrap();

    let base_src_address = mem.read_u32(DMA_SAD + i*0xC) & 0xFFFFFFF; // top bits ignored
    let base_dst_address = mem.read_u32(DMA_DAD + i*0xC) & 0xFFFFFFF; // top bits ignored
    let cnt_l = mem.read_u16(DMA_AMOUNT + i *0xC) as u32;
    let amount = match i {
        3 => match cnt_l {
            0 => 0x10000,
            _ => cnt_l,
        }
        _ => match cnt_l {
            0 => 0x4000,
            _ => cnt_l,
        }
    };

    let dst_ctrl = (cnt >> 5) & 0x3;
    let src_ctrl = (cnt >> 7) & 0x3;

    let repeat = (cnt >> 9) & 1 == 1;
    let quantities = (cnt >> 10) & 1 == 1;
    let _drq = (cnt >> 11) & 1 == 1; // this isn't possible to implement????

    let dma_start = (cnt >> 12) & 0x3;
    let irq_call = (cnt >> 14) & 1 == 1;

    let dispstat = mem.read_u16(0x4000004);
    match dma_start {
        0 => {}
        1 => if (dispstat >> 0) & 1 == 0 { return false; }
        2 => if (dispstat >> 1) & 1 == 0 { return false; }
        3 => {}
        _ => unreachable!(),
    }

    let done_already = mem.dma_completions[i as usize];
    let src_address = match src_ctrl {
        0 => base_src_address + done_already,
        1 => base_src_address - done_already,
        2 => base_src_address,
        _ => unreachable!("invalid DMA transfer"),
    };

    let dst_address = match dst_ctrl {
        0 => base_dst_address + done_already,
        1 => base_dst_address - done_already,
        2 => base_dst_address,
        3 => base_dst_address + done_already,
        _ => unreachable!(),
    };

    match quantities {
        true => {
            // 32-bit
            let read = mem.read_u32(src_address);
            mem.write_u32(dst_address, read);

            mem.dma_completions[i as usize] += 4;
        }
        false => {
            // 16-bit
            let read = mem.read_u16(src_address);
            mem.write_u16(dst_address, read);

            mem.dma_completions[i as usize] += 2;
        }
    }

    // DMA is finished
    let final_amount = match quantities {
        true => amount * 4,
        false => amount * 2,
    };

    if mem.dma_completions[i as usize] >= final_amount {
        if irq_call {
            let mut i_flag = mem.read_u16(0x4000202);
            i_flag |= 1 << (8 + i);

            mem.write_u16(0x4000202, i_flag);
        }

        mem.dma_completions[i as usize] = 0;
        if repeat {
            return true;
        }

        // clear the top bit
        mem.write_io(DMA_CONTROL + i*0xC, cnt & 0x7FFF);
        return false;
    }

    return true;
}


const BASE_TIMER_ADDRESS: u32 = 0x4000100;
pub fn update_timer(memory: &mut Memory, old_cycles: &mut u32, new_cycles: u32) {
    let total_cycles = *old_cycles + new_cycles;
    let mut prev_cascade = false;

    for timer in 0..=3 {
        let timer_address = BASE_TIMER_ADDRESS + (timer * 4);
        let control = memory.read_u16(timer_address + 2);

        let timer_enable = (control >> 7) & 1 == 1;
        if !timer_enable {
            prev_cascade = false;
            continue;
        }

        let frequency_bits = control & 0b11;
        let frequency = match frequency_bits {
            0b00 => 1,
            0b01 => 64,
            0b10 => 256,
            0b11 => 1024,
            _ => unreachable!()
        };

        let timer_cycles = memory.read_u16(timer_address) as u32;

        let cascade_timer = (control >> 2) & 1 == 1;
        let (new_timer_cycles, overflow) = match cascade_timer {
            true => old_cycles.overflowing_add(prev_cascade as u32),
            false => {
                let cycles_to_add = (timer_cycles / frequency).wrapping_sub(total_cycles / frequency);
                old_cycles.overflowing_add(cycles_to_add as u32)
            }
        };
        prev_cascade = overflow;

        let interrupt_flag = (control >> 6) & 1 == 1;
        
        // we need to call the interrupt
        if overflow && interrupt_flag {
            let mut interrupt_flag = memory.read_u16(0x4000202);
            interrupt_flag |= 1 << (timer + 3);

            memory.write_io(0x4000202, interrupt_flag);
        }

        match overflow {
            true => memory.write_io(timer_address, memory.timer_resets[timer as usize]),
            false => memory.write_io(timer_address, new_timer_cycles as u16),
        }
    }

    *old_cycles = total_cycles % 1024;
}

pub fn create_memory(file_name: &str) -> Box<Memory> {
    let file = match std::fs::read(file_name) {
        Err(e) => panic!("invalid file provided => {e:?}"),
        Ok(f) => f,
    };

    Box::new(Memory {
        ewram: [0; EWRAM_LENGTH],
        iwram: [0; IWRAM_LENGTH],
        vram: [0; VRAM_LENGTH],
        io_reg: [0; IO_REG_LENGTH],
        obj_pall: [0; OBJ_PALL_LENGTH],
        oam: [0; OAM_LENGTH],
        gp_rom: file,
        sram: [0; SRAM_MAX_LENGTH],

        timer_resets: [0; 4],
        dma_completions: [0; 4],
    })
}