const BIOS: &[u8; 0x4000] = include_bytes!("../bios/bios.bin");

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
fn little_split_u32(a: u32) -> (u8, u8, u8, u8) {
    return (a as u8, (a >> 8) as u8, (a >> 16) as u8, (a >> 24) as u8)
}

#[inline]
fn split_memory_address(address: u32) -> (u32, usize) {
    ((address >> 24) & 0xF, (address & 0xFFFFF) as usize)
}

const DMA_SAD: u32 = 0x40000B0;
const DMA_DAD: u32 = 0x40000B4;
const DMA_COUNT: u32 = 0x40000B8;
const DMA_CNT: u32 = 0x40000BA;

const EWRAM_LENGTH: usize = 0x40000;
const IWRAM_LENGTH: usize = 0x8000;
const VRAM_LENGTH: usize = 0x18000;
const IO_REG_LENGTH: usize = 0x400;
const OBJ_PALL_LENGTH: usize = 0x400;
const OAM_LENGTH: usize = 0x400;
/// I guess that it is possible to store all of the stores as 
/// their respective bus lengths, but it may mess with the little-endianness of the
/// machine. May perform tests if it works
pub struct Memory {
    ewram: [u8; EWRAM_LENGTH], // WRAM - On-board Work RAM
    iwram: [u8; IWRAM_LENGTH],  // WRAM - On-chip Work RAM
    vram: [u8; VRAM_LENGTH], // 96 KB - 16 bit bus
    io_reg: [u8; IO_REG_LENGTH],
    obj_pall: [u8; OBJ_PALL_LENGTH],
    oam: [u8; OAM_LENGTH],
    gp_rom: Vec<u8>,

    timer_resets: [u16; 4],
}
impl Memory {
    // since DMA takes several cycles, its best to just have it be its own thing
    pub fn dma_tick(&mut self) {
        let mut dma_transfer = None;
        for i in 0..=3 {
            let cnt = self.read_u16(DMA_CNT + i*8);
            let is_on = (cnt >> 15) & 1 == 1;
            if is_on {
                dma_transfer = Some((i, cnt));
                break;
            }
        }

        // no dma transfer active rn
        if let None = dma_transfer {
            return;
        }

        let (dma, cnt) = dma_transfer.unwrap();
        println!("{dma}. {cnt}");
    }

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
                return self.vram[base];
            }
            0x7 => self.oam[low_add % OAM_LENGTH],
            _ => {
                if low_add >= self.gp_rom.len() {
                    return 0x00;
                }
                self.gp_rom[low_add]
            },
        }
    }

    pub fn read_u16(&self, address: u32) -> u16 {
        let base_address = (address / 2) * 2;

        lil_end_combine_u16(
            self.read_u8(base_address + ((address + 0) % 2)), 
            self.read_u8(base_address + ((address + 1) % 2))
        )
    }

    pub fn read_u32(&self, address: u32) -> u32 {
        let base_address = (address / 4) * 4;

        lil_end_combine_u32(
            self.read_u8(base_address + ((address + 0) % 4)), 
            self.read_u8(base_address + ((address + 1) % 4)), 
            self.read_u8(base_address + ((address + 2) % 4)), 
            self.read_u8(base_address + ((address + 3) % 4)),
        )
    }

    fn checked_write_u8(&mut self, address: u32, data: u8, is_8_bit: bool) {
        if address == 0x4000202 || address == 0x4000203 {
            self.obj_pall[address as usize - 0x4000000] &= !data;
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
            0x2 => self.ewram[low_add % self.ewram.len()] = data,
            0x3 => self.iwram[low_add % self.iwram.len()] = data,
            0x4 => self.io_reg[low_add % self.io_reg.len()] = data,
            0x5 => self.obj_pall[low_add % self.obj_pall.len()] = data,
            0x6 => self.vram[low_add % self.vram.len()] = data,
            0x7 => self.oam[low_add % self.oam.len()] = data,
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
        let split = little_split_u32(data);
        let address = address & !(0b11);

        self.checked_write_u8(address + 0, split.0, false);
        self.checked_write_u8(address + 1, split.1, false);
        self.checked_write_u8(address + 2, split.2, false);
        self.checked_write_u8(address + 3, split.3, false);
    }

    /// avoids all of the checks, used just by the other sub-systems
    pub fn write_io(&mut self, address: u32, data: u16) {
        let address = address as usize - 0x4000000;
        let split = lil_end_split_u16(data);

        self.io_reg[address + 0] = split.0;
        self.io_reg[address + 1] = split.1;
    }
}
fn is_in_video_memory(upp_add: u32) -> bool {
    return (upp_add == 0x5) | (upp_add == 0x6) | (upp_add == 0x7);
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

        let frequency;
        let frequency_bits = control & 0b11;
        match frequency_bits {
            0b00 => frequency = 1,
            0b01 => frequency = 64,
            0b10 => frequency = 256,
            0b11 => frequency = 1024,
            _ => unreachable!()
        }

        let timer_cycles = memory.read_u16(timer_address) as u32;

        let cascade_timer = (control >> 2) & 1 == 1;
        let (new_timer_cycles, overflow) = match cascade_timer {
            true => old_cycles.overflowing_add(prev_cascade as u32),
            false => {
                let cycles_to_add = (timer_cycles / frequency) - (total_cycles / frequency);
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

pub fn create_memory(file_name: &str) -> Memory {
    let file = match std::fs::read(file_name) {
        Err(e) => panic!("invalid file provided => {e:?}"),
        Ok(f) => f,
    };

    Memory {
        ewram: [0; 0x40000],
        iwram: [0; 0x8000],
        vram: [0; 0x18000],
        io_reg: [0; 0x400],
        obj_pall: [0; 0x400],
        oam: [0; 0x400],
        gp_rom: file,
        timer_resets: [0; 4],
    }
}