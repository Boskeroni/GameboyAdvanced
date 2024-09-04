const BIOS: &[u8; 0x4000] = include_bytes!("../bios/gba_bios.bin");

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


/// I guess that it is possible to store all of the stores as 
/// their respective bus lengths, but it may mess with the little-endianness of the
/// machine. May perform tests if it works
pub struct Memory {
    ewram: [u8; 0x40000], // WRAM - On-board Work RAM
    iwram: [u8; 0x8000],  // WRAM - On-chip Work RAM
    vram: [u8; 0x18000], // 96 KB - 16 bit bus
    io_reg: [u8; 0x400],
    obj_pall: [u8; 0x400],
    oam: [u8; 0x400],
    gp_rom: Vec<u8>,
}
impl Memory {
    pub fn read_u8(&self, address: u32) -> u8 {
        let (upp_add, low_add) = split_memory_address(address);

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
        let (upp_add, low_add) = split_memory_address(address);
        assert!(low_add & 1 != 1, "A[0] must be equal to 0 when reading half-words");

        match upp_add {
            0x0 => lil_end_combine_u16(BIOS[low_add], BIOS[low_add+1]),
            0x2 => lil_end_combine_u16(self.ewram[low_add], self.ewram[low_add+1]),
            0x3 => lil_end_combine_u16(self.iwram[low_add], self.iwram[low_add+1]),
            0x4 => lil_end_combine_u16(self.io_reg[low_add], self.io_reg[low_add+1]),
            0x5 => lil_end_combine_u16(self.obj_pall[low_add], self.obj_pall[low_add+1]),
            0x6 => lil_end_combine_u16(self.vram[low_add], self.vram[low_add+1]),
            0x7 => lil_end_combine_u16(self.oam[low_add], self.oam[low_add+1]),
            _ => {
                let gp_address = address as usize - 0x8000000; 
                lil_end_combine_u16(self.gp_rom[gp_address], self.gp_rom[gp_address + 1])      
            }
        }
    }

    pub fn read_u32(&self, address: u32) -> u32 {
        // the real low_add doesnt include the bottom 2 bits
        let (upp_add, raw_low_add) = split_memory_address(address);
        let low_add = raw_low_add & 0xFFFFC;

        let raw_reading = match upp_add {
            0x0 => lil_end_combine_u32(BIOS[low_add], BIOS[low_add+1], BIOS[low_add+2], BIOS[low_add+3]),
            0x2 => lil_end_combine_u32(self.ewram[low_add], self.ewram[low_add+1], self.ewram[low_add+2], self.ewram[low_add+3]),
            0x3 => lil_end_combine_u32(self.iwram[low_add], self.iwram[low_add+1], self.iwram[low_add+2], self.iwram[low_add+3]),
            0x4 => lil_end_combine_u32(self.io_reg[low_add], self.io_reg[low_add+1], self.io_reg[low_add+2], self.io_reg[low_add+3]),
            0x5 => lil_end_combine_u32(self.obj_pall[low_add], self.obj_pall[low_add+1], self.obj_pall[low_add+2], self.obj_pall[low_add+3]),
            0x6 => lil_end_combine_u32(self.vram[low_add], self.vram[low_add+1], self.vram[low_add+2], self.vram[low_add+3]),
            0x7 => lil_end_combine_u32(self.oam[low_add], self.oam[low_add+1], self.oam[low_add+2], self.oam[low_add+3]),
            _ => {
                let gp_address = address as usize - 0x8000000; 
                lil_end_combine_u32(self.gp_rom[gp_address], self.gp_rom[gp_address + 1], self.gp_rom[gp_address + 2], self.gp_rom[gp_address+3])      
            },
        };

        // i am not going to do any rotating until I know it is necessary
        return raw_reading;
    }

    pub fn write_u8(&mut self, address: u32, data: u8) {
        let (upp_add, low_add) = split_memory_address(address);

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
        let upp_add = (address >> 24) & 0xF;
        let low_add = (address & 0xFFFFC) as usize;

        let split = lil_end_split_u16(data);
        match upp_add {
            0x0 => panic!("cannot make a write to the BIOS"),
            0x2 => {self.ewram[low_add] = split.0; self.ewram[low_add+1] = split.1},
            0x3 => {self.iwram[low_add] = split.0; self.iwram[low_add+1] = split.1},
            0x4 => {self.io_reg[low_add] = split.0; self.io_reg[low_add+1] = split.1}
            0x5 => {self.obj_pall[low_add] = split.0; self.obj_pall[low_add+1] = split.1},
            0x6 => {self.vram[low_add] = split.0; self.vram[low_add+1] = split.1},
            0x7 => {self.oam[low_add] = split.0; self.oam[low_add+1] = split.1},
            _ => todo!("I havent implemented external memory yet")
        }
    }

    pub fn write_u32(&mut self, address: u32, data: u32) {
        let upp_add = (address >> 24) & 0xF;
        // bottom 2 bits are ignored when writing words
        let low_add = (address & 0xFFFFC) as usize;
        let split = little_split_u32(data);

        match upp_add {
            0x0 => panic!("cannot make a write to the BIOS"),
            0x2 => {self.ewram[low_add] = split.0; self.ewram[low_add+1] = split.1; self.ewram[low_add+2] = split.2; self.ewram[low_add+3] = split.3},
            0x3 => {self.iwram[low_add] = split.0; self.iwram[low_add+1] = split.1; self.iwram[low_add+2] = split.2; self.iwram[low_add+3] = split.3},
            0x4 => {self.io_reg[low_add] = split.0; self.io_reg[low_add+1] = split.1; self.io_reg[low_add+2] = split.2; self.io_reg[low_add+3] = split.3},
            0x5 => {self.obj_pall[low_add] = split.0; self.obj_pall[low_add+1] = split.1; self.obj_pall[low_add+2] = split.2; self.obj_pall[low_add+3] = split.3},
            0x6 => {self.vram[low_add] = split.0; self.vram[low_add+1] = split.1; self.vram[low_add+2] = split.2; self.vram[low_add+3] = split.3},
            0x7 => {self.oam[low_add] = split.0; self.oam[low_add+1] = split.1; self.oam[low_add+2] = split.2; self.oam[low_add+3] = split.3},
            _ => todo!("I haven't implemented external memory yet"),
        }
    }
}

pub fn create_memory(file_name: &str) -> Memory {
    let file = match std::fs::read(file_name) {
        Err(e) => panic!("invalid file provided => {e:?}"),
        Ok(f) => f,
    };

    let memory = Memory {
        ewram: [0; 0x40000],
        iwram: [0; 0x8000],
        vram: [0; 0x18000],
        io_reg: [0; 0x400],
        obj_pall: [0; 0x400],
        oam: [0; 0x400],
        gp_rom: file,
    };
    
    memory
}