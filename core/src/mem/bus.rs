use crate::mem::is_in_video_memory;
use crate::mem::lil_end_combine_u16;
use crate::mem::lil_end_combine_u32;
use crate::mem::lil_end_split_u16;
use crate::mem::lil_end_split_u32;
use crate::mem::memory::InternalMemory;
use crate::mem::split_memory_address;

pub trait CpuInterface {
    fn read_u8(&self, address: u32) -> u8;
    fn read_u16(&self, address: u32) -> u16;
    fn read_u32_unrotated(&self, address: u32) -> u32;
    fn read_u32_rotated(&self, address: u32) -> u32;

    fn write_u8(&mut self, address: u32, data: u8);
    fn write_u16(&mut self, address: u32, data: u16);
    fn write_u32(&mut self, address: u32, data: u32);
}
pub trait PpuInterface {
    fn read_vram_u8(&self, address: u32) -> u8;
    fn read_vram_u16(&self, address: u32) -> u16;
    fn read_vram_u32(&self, address: u32) -> u32;

    fn write_vram_u16(&mut self, address: u32, data: u16);
}

// the area that it writes/reads from can affect
// what to do with the data so this is a nice abstraction
#[derive(PartialEq)]
pub enum MemoryRegion {
    Bios,
    WramBoard,
    WramChip,
    IoReg,
    Rom,
    Sram,
}
impl MemoryRegion {
    fn from_pc(pc: u32) -> MemoryRegion {
        let (up, _) = split_memory_address(pc);
        use MemoryRegion::*;
        match up {
            0 => Bios,
            2 => WramBoard,
            3 => WramChip,
            4 => IoReg,
            8..=0xD => Rom,
            0xE => Sram,
            _ => unreachable!("code should never execute here"),
        }
    }
}

pub struct Bus {
    last_bios_fetch: u32,
    pc_fetched_area: MemoryRegion,
    last_fetched_opcode: u32,
    pub mem: Box<InternalMemory>,
    should_halt_cpu: bool,
}

impl Bus {
    pub fn new(mem: Box<InternalMemory>, from_bios: bool) -> Self {
        let mut default = Self {
            last_bios_fetch: 0x0,
            pc_fetched_area: MemoryRegion::Bios,
            last_fetched_opcode: 0x0,
            mem,
            should_halt_cpu: false,
        };

        // starting from the bios
        if from_bios {
            return default;
        }

        let last_fetched = default.fetch_arm_opcode(0xDC + 8);
        default.last_bios_fetch = last_fetched;
        default.last_fetched_opcode = last_fetched;
        return default;
    }

    pub fn fetch_arm_opcode(&mut self, pc: u32) -> u32 {
        self.pc_fetched_area = MemoryRegion::from_pc(pc);
        let opcode = self.mem.sys_read_u32(pc);
        if let MemoryRegion::Bios = self.pc_fetched_area {
            self.last_bios_fetch = opcode;
        }
        
        return opcode;
    }
    pub fn fetch_thumb_opcode(&mut self, pc: u32) -> u16 {
        self.pc_fetched_area = MemoryRegion::from_pc(pc);
        let opcode = self.mem.sys_read_u16(pc);
        if let MemoryRegion::Bios = self.pc_fetched_area {
            self.last_bios_fetch = opcode as u32;
        }

        return opcode;
    }
    
    pub fn sys_write_u16(&mut self, address: u32, data: u16) {
        self.mem.sys_write_u16(address, data);
    }

    pub fn should_halt_cpu(&mut self) -> bool {
        let stored = self.should_halt_cpu;
        self.should_halt_cpu = false;
        return stored;
    }

    pub fn cpu_read(&self, address: u32) -> u8 {
        if let Some(data) = self.mem.cpu_read(address) {
            return data;
        }
        
        // reaching here means the address was invalid
        // and so most recent opcode fetch should be done
        let rotation_amount = (address & 0x3) * 8;
        let rotated_op = self.last_fetched_opcode.rotate_right(rotation_amount as u32);
        return rotated_op as u8;
    }
    pub fn cpu_write(&mut self, address: u32, data: u8, is_8_bit: bool) {
        self.mem.cpu_write(address, data, is_8_bit);
    }
}

impl CpuInterface for Bus {
    fn read_u8(&self, address: u32) -> u8 {
        self.cpu_read(address)
    }
    fn read_u16(&self, address: u32) -> u16 {
        let base_address = address & !(0b1);

        lil_end_combine_u16(
            self.cpu_read(base_address + 0), 
            self.cpu_read(base_address + 1),
        )
    }

    // I am not too sure if these two functions are needed
    // i may eventually get ride of them
    fn read_u32_unrotated(&self, address: u32) -> u32 {
        let base_address = address & !(0b11);

        lil_end_combine_u32(
            self.cpu_read(base_address + 0), 
            self.cpu_read(base_address + 1), 
            self.cpu_read(base_address + 2), 
            self.cpu_read(base_address + 3),
        )
    }
    fn read_u32_rotated(&self, address: u32) -> u32 {
        self.read_u32_unrotated(address).rotate_right((address & 0b11) * 8)
    }

    fn write_u16(&mut self, address: u32, data: u16) {
        let split = lil_end_split_u16(data);
        let address = address & !(0b1);

        self.cpu_write(address + 0, split.0, false);
        self.cpu_write(address + 1, split.1, false);
    }
    fn write_u32(&mut self, address: u32, data: u32) {
        let split = lil_end_split_u32(data);
        let address = address & !(0b11);

        self.cpu_write(address + 0, split.0, false);
        self.cpu_write(address + 1, split.1, false);
        self.cpu_write(address + 2, split.2, false);
        self.cpu_write(address + 3, split.3, false);
 
    }
    fn write_u8(&mut self, address: u32, data: u8) {
        if address == 0x4000301 {
            self.should_halt_cpu = true;
            return;
        }

        self.cpu_write(address, data, true);
    }
}
impl PpuInterface for Bus {
    fn read_vram_u16(&self, address: u32) -> u16 {
        let (upp, _) = split_memory_address(address);

        assert!(is_in_video_memory(upp) || upp == 0x4);
        self.mem.sys_read_u16(address)
    }
    fn read_vram_u32(&self, address: u32) -> u32 {
        let (upp, _) = split_memory_address(address);

        assert!(is_in_video_memory(upp) || upp == 0x4);
        self.mem.sys_read_u32(address)
    }
    fn read_vram_u8(&self, address: u32) -> u8 {
        let (upp, _) = split_memory_address(address);

        assert!(is_in_video_memory(upp) || upp == 0x4);
        self.mem.sys_read_u8(address)
    }
    fn write_vram_u16(&mut self, address: u32, data: u16) {
        let (upp, _) = split_memory_address(address);

        assert!(is_in_video_memory(upp) || upp == 0x4);
        self.mem.sys_write_u16(address, data);
    }
}