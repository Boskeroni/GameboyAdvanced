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

#[inline]
/// input => 0bDDDDDDDDCCCCCCCCBBBBBBBBAAAAAAAA
/// output => (0bAAAAAAAA, 0bBBBBBBBB, 0bCCCCCCCC, 0bDDDDDDDD)
fn little_split_u32(a: u32) -> (u8, u8, u8, u8) {
    return (a as u8, (a >> 8) as u8, (a >> 16) as u8, (a >> 24) as u8)
}

#[inline]
fn split_memory_address(address: u32) -> (u32, usize) {
    ((address >> 24) & 0xF, (address & 0xFFFFF) as usize)
}


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
    pub fn read_u8(&self, address: u32) -> u8 {
        let (upp_add, low_add) = split_memory_address(address);

        if !self.check_valid_address(upp_add, low_add) {
            panic!("out of bounds memory read {address:X}");
        }

        match upp_add {
            0x0 => BIOS[low_add],
            0x2 => self.ewram[low_add],
            0x3 => self.iwram[low_add],
            0x4 => self.io_reg[low_add],
            0x5 => self.obj_pall[low_add],
            0x6 => self.vram[low_add],
            0x7 => self.oam[low_add],
            _ => self.gp_rom[address as usize - 0x8000000],
        }
    }

    pub fn read_u16(&self, address: u32) -> u16 {
        lil_end_combine_u16(
            self.read_u8(address + 0), 
            self.read_u8(address + 1)
        )
    }

    pub fn read_u32(&self, address: u32) -> u32 {
        lil_end_combine_u32(
            self.read_u8(address + 0), 
            self.read_u8(address + 1), 
            self.read_u8(address + 2), 
            self.read_u8(address + 3),
        )
    }

    pub fn write_u8(&mut self, address: u32, data: u8) {
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
        
        if !self.check_valid_address(upp_add, low_add) {
            return;
        }
        match upp_add {
            0x0 => panic!("cannot make a write to the BIOS"),
            0x2 => self.ewram[low_add] = data,
            0x3 => self.iwram[low_add] = data,
            0x4 => self.io_reg[low_add] = data,
            0x5 => self.obj_pall[low_add] = data,
            0x6 => self.vram[low_add] = data,
            0x7 => self.oam[low_add] = data,
            _ => todo!("I haven't implemented display memory yet"),
        };
    }

    pub fn write_u16(&mut self, address: u32, data: u16) {
        let split = lil_end_split_u16(data);

        self.write_u8(address + 0, split.0);
        self.write_u8(address + 1, split.1);
    }

    pub fn write_u32(&mut self, address: u32, data: u32) {
        let split = little_split_u32(data);
        let address = address & !(0b11);

        self.write_u8(address + 0, split.0);
        self.write_u8(address + 1, split.1);
        self.write_u8(address + 2, split.2);
        self.write_u8(address + 3, split.3);
    }

    fn write_io(&mut self, address: u32, data: u16) {
        let address = address as usize - 0x4000000;
        let split = lil_end_split_u16(data);

        self.io_reg[address] = split.0;
        self.io_reg[address + 1] = split.1;
    }
    fn check_valid_address(&self, upper: u32, lower: usize) -> bool {
        match upper {
            0x0 => lower < BIOS.len(),
            0x2 => lower < EWRAM_LENGTH,
            0x3 => lower < IWRAM_LENGTH,
            0x4 => lower < IO_REG_LENGTH,
            0x5 => lower < OBJ_PALL_LENGTH,
            0x6 => lower < VRAM_LENGTH,
            0x7 => lower < OAM_LENGTH,
            0x8 => lower < self.gp_rom.len(),
            _ => unreachable!(),
        }
    }
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