use crate::mem::*;

// this has been acquired legally
pub const BIOS: &[u8; 0x4000] = include_bytes!("bios.bin");

pub struct MemLengths;
impl MemLengths {
    pub const EWRAM: usize = 0x40000;
    pub const IWRAM: usize = 0x8000;
    pub const IO: usize = 0x3FF;
    pub const OBJ: usize = 0x400;
    pub const VRAM: usize = 0x18000;
    pub const OAM: usize = 0x400;
    pub const MAX_SRAM: usize = 0x10000;
}

pub fn create_memory(file_name: &str) -> Box<InternalMemory> {
    let file = match std::fs::read(file_name) {
        Err(e) => panic!("invalid file provided => {e:?}"),
        Ok(f) => f,
    };

    Box::new(InternalMemory {
        ewram: [0; MemLengths::EWRAM],
        iwram: [0; MemLengths::IWRAM],
        vram: [0; MemLengths::VRAM],
        io_reg: [0; MemLengths::IO],
        obj_pall: [0; MemLengths::OBJ],
        oam: [0; MemLengths::OAM],
        rom: file,
        sram: [0; MemLengths::MAX_SRAM],

        timer_reload_values: [0; 4],
        dma_completions: [0; 4],
    })
}

pub struct InternalMemory {
    pub ewram: [u8; MemLengths::EWRAM], // WRAM - On-board Work RAM
    pub iwram: [u8; MemLengths::IWRAM],  // WRAM - On-chip Work RAM
    pub vram: [u8; MemLengths::VRAM], // 96 KB - 16 bit bus
    pub io_reg: [u8; MemLengths::IO],
    pub obj_pall: [u8; MemLengths::OBJ],
    pub oam: [u8; MemLengths::OAM],
    pub rom: Vec<u8>,
    pub sram: [u8; MemLengths::MAX_SRAM],

    timer_reload_values: [u16; 4],
    dma_completions: [u32; 4],
}
impl InternalMemory {
    pub fn cpu_read(&self, address: u32) -> Option<u8> {
        if has_read_lock(address) {
            return None;
        }

        let (upp, low) = split_memory_address(address);
        if upp == 0x0 && low >= BIOS.len() {
            return None;
        }
        if upp > 0xE {
            return None;
        }
    
        return Some(self.sys_read_u8(address));
    }
    pub fn cpu_write(&mut self, address: u32, data: u8, is_8_bit: bool) {
        if has_write_lock(address) {
            return;
        }
        if address == 0x4000202 || address == 0x4000203 {
            self.io_reg[address as usize - 0x4000000] &= !data;
            return;
        }

        // writing to a timer register
        if address >= 0x4000100 && address <= 0x400010E {
            let timer_specified = (address - 0x4000100) / 4;
            // check if its enabling the timer
            if address % 4 == 2 {
                let enable_bit = data >> 7 & 1 == 1;
                if enable_bit {
                    let timer_reload = self.timer_reload_values[timer_specified as usize];
                    self.sys_write_u16(address - 2, timer_reload);
                }
                self.sys_write_u16(address, data as u16);
                return;
            }
            if address % 4 >= 2 {
                return;
            }
            
            // this rounds down anyways which is good
            let write_upper_byte = address & 1 == 1;
            let data = data as u16;

            let timer_reset = &mut self.timer_reload_values[timer_specified as usize];
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
            let halfword_aligned = address & !0b1;
            self.cpu_write(halfword_aligned + 0, data, false);
            self.cpu_write(halfword_aligned + 1, data, false);
            return;
        }

        self.sys_write_u8(address, data);
    }

    /// this provides unchecked reading,
    /// so should only be used by the PPU (which technically owns all
    /// of its memory and registers)
    pub fn sys_read_u8(&self, address: u32) -> u8 {
        let (upp, low) = split_memory_address(address);

        match upp {
            0x0 => return BIOS[low % BIOS.len()],
            0x2 => return self.ewram[low % MemLengths::EWRAM],
            0x3 => return self.iwram[low % MemLengths::IWRAM],
            0x4 => return self.io_reg[low % MemLengths::IO],
            0x5 => return self.obj_pall[low % MemLengths::OBJ],
            0x6 => {
                let base = low & 0x1FFFF;
                if base >= 0x10000 {
                    return self.vram[0x10000 + (base & 0x7FFF)];
                }
                return self.vram[base];
            }
            0x7 => return self.oam[low % MemLengths::OAM],
            0x8..=0xD => {
                if upp % 2 == 1 {
                    return self.rom[(low + 0x1000000) % self.rom.len()];
                }
                return self.rom[low % self.rom.len()];
            }
            0xE => return self.sram[low % MemLengths::MAX_SRAM],
            _ => panic!("this should never be read from"),
        }
    }
    pub fn sys_read_u16(&self, address: u32) -> u16 {
        let base = address & !(0b1);

        lil_end_combine_u16(
            self.sys_read_u8(base + 0), 
            self.sys_read_u8(base + 1),
        )
    }
    pub fn sys_read_u32(&self, address: u32) -> u32 {
        let base = address & !(0b11);

        lil_end_combine_u32(
            self.sys_read_u8(base + 0), 
            self.sys_read_u8(base + 1), 
            self.sys_read_u8(base + 2), 
            self.sys_read_u8(base + 3),
        )
    }
    pub fn sys_write_u16(&mut self, address: u32, data: u16) {
        let base = address & !(0b1);
        let split = lil_end_split_u16(data);

        self.sys_write_u8(base + 0, split.0);
        self.sys_write_u8(base + 1, split.1);
    }
    pub fn sys_write_u8(&mut self, address: u32, data: u8) {
        let (upp, low) = split_memory_address(address);
        match upp {
            0x0 => panic!("cannot make a write to the BIOS"),
            0x2 => self.ewram[low % MemLengths::EWRAM] = data,
            0x3 => self.iwram[low % MemLengths::IWRAM] = data,
            0x4 => self.io_reg[low % MemLengths::IO] = data,
            0x5 => self.obj_pall[low % MemLengths::OBJ] = data,
            0x6 => {
                // 64k-32k (then the 32k is mirrored again) (then everything is mirrored again)
                let base = low % 0x20000;
                if base >= 0x10000 {
                    self.vram[0x10000 + (base % 0x8000)] = data;
                    return;
                }
                self.vram[base] = data;
            }
            0x7 => self.oam[low % MemLengths::OAM] = data,
            0xE => self.sram[low % MemLengths::MAX_SRAM] = data,
            _ => println!("cannot write to ROM {address:X}, {data:X}"),
        };
    }

}
// since DMA takes several cycles, its best to just have it be its own thing
pub enum DMARegisters {
    SAD = 0x40000B0,
    DAD = 0x40000B4,
    Amount = 0x40000B8,
    Control = 0x40000BA,
}
pub fn dma_tick(mem: &mut Box<InternalMemory>) -> bool {
    let mut dma_transfer = None;
    for i in 0..=3 {
        let cnt = mem.sys_read_u16(DMARegisters::Control as u32 + (i*0xC));
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

    let base_src_address = mem.sys_read_u32(DMARegisters::SAD as u32 + (i*0xC)) & 0x0FFFFFFF; // top bits ignored
    let base_dst_address = mem.sys_read_u32(DMARegisters::DAD as u32 + (i*0xC)) & 0x0FFFFFFF; // top bits ignored
    let cnt_l = mem.sys_read_u16(DMARegisters::Amount as u32 + (i*0xC)) as u32;
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

    let dispstat = mem.sys_read_u16(0x4000004);
    match dma_start {
        0 => {}
        1 => if (dispstat >> 0) & 1 == 0 { return false; }
        2 => if (dispstat >> 1) & 1 == 0 { return false; }
        3 => {
            // special so must be false
            assert!(base_dst_address == 0x40000A0 || base_dst_address == 0x40000A4);
            // turn that shit off :P
            mem.sys_write_u16(DMARegisters::Control as u32 + i*0xC, cnt & 0x7FFF);
            return false;
        }
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
            let read = mem.sys_read_u32(src_address);
            mem.sys_write_u16(dst_address + 0, read as u16);
            mem.sys_write_u16(dst_address + 2, (read >> 16) as u16);

            mem.dma_completions[i as usize] += 4;
        }
        false => {
            // 16-bit
            let read = mem.sys_read_u16(src_address);
            mem.sys_write_u16(dst_address, read);

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
            let mut i_flag = mem.sys_read_u16(0x4000202);
            i_flag |= 1 << (8 + i);

            mem.sys_write_u16(0x4000202, i_flag);
        }

        mem.dma_completions[i as usize] = 0;
        if repeat {
            return true;
        }

        // clear the top bit
        mem.sys_write_u16(DMARegisters::Control as u32 + i*0xC, cnt & 0x7FFF);
        return false;
    }

    return true;
}


const BASE_TIMER_ADDRESS: u32 = 0x4000100;
const FREQUENCY: [u32; 4] = [1, 64, 256, 1024];
pub fn update_timer(memory: &mut Box<InternalMemory>, old_cycles: &mut u32, new_cycles: u32) {
    let total_cycles = *old_cycles + new_cycles;
    let mut prev_cascade = false;

    for timer in 0..=3 {
        // the address
        let timer_address = BASE_TIMER_ADDRESS + (timer * 4);
        let control = memory.sys_read_u16(timer_address + 2);

        // its not turned on
        let timer_enable = (control >> 7) & 1 == 1;
        if !timer_enable {
            prev_cascade = false;
            continue;
        }

        // get the frequency
        let frequency_bits = control & 0b11;
        let frequency = FREQUENCY[frequency_bits as usize];

        // cycles already done
        let timer_cycles = memory.sys_read_u16(timer_address);

        let count_up_timer = (control >> 2) & 1 == 1;
        let (new_timer_cycles, overflow) = match count_up_timer {
            true => timer_cycles.overflowing_add(prev_cascade as u16),
            false => {
                // everytime the total cycles passes a multiple of the frequency, then add 1 no?
                let cycles_to_add = total_cycles/frequency - *old_cycles/frequency;
                timer_cycles.overflowing_add(cycles_to_add as u16)
            }
        };
        prev_cascade = overflow;

        let interrupt_flag = (control >> 6) & 1 == 1;
        
        // we need to call the interrupt
        if overflow && interrupt_flag {
            let mut interrupt_flag = memory.sys_read_u16(0x4000202);
            interrupt_flag |= 1 << (timer + 3);

            memory.sys_write_u16(0x4000202, interrupt_flag);
        }

        match overflow {
            true => memory.sys_write_u16(timer_address, memory.timer_reload_values[timer as usize]),
            false => memory.sys_write_u16(timer_address, new_timer_cycles as u16),
        }
    }

    *old_cycles = total_cycles % 1024;
}