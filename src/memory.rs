use crate::{little_endian_u8_u16, little_endian_u8_u32};

const BIOS: &[u8; 0x4000] = include_bytes!("../bios/gba_bios.bin");

/// the different memory locations can and will return differently
/// sized values, so its best to just store them in an enum probably
pub enum MemoryRead {
    Byte(u8),
    Halfword(u16),
    Word(u32),
}

/// I guess that it is possible to store all of the stores as 
/// their respective bus lengths, but it may mess with the little-endianness of the
/// machine. May perform tests if it works
pub struct GeneralMemory {
    ewram: [u8; 0x40000], // 256KB - 16 bit bus
    iwram: [u8; 0x8000],  // 32KB - 32 bit bus
    vram: [u8; 0x18000], // 96 KB - 16 bit bus
    io_registers: [u8; 0x3FF],
    gamepak_rom: Vec<u8>, // variable size - 16 bit bus
    gamepak_ram: Vec<u8>, // variable size - 8 bit bus 
}
impl GeneralMemory {
    pub fn read(&self, address: u32) -> MemoryRead {
        // only the bottom 28 bits are kept
        let address = (address & 0x0FFFFFFF) as usize;
        use MemoryRead::*;

        // bits 24 - 28
        match address >> 24 {
            0x00 => { // bios has a 32-bit bus
                let data = little_endian_u8_u32(BIOS[address], BIOS[address+1], BIOS[address+2], BIOS[address+3]);
                Word(data)
            },
            0x02 => { // on board WRAM has a 32-bit bus
                let address = address - 0x2000000;
                let data = little_endian_u8_u16(self.ewram[address], self.ewram[address+1]);
                Halfword(data)
            },
            0x03 => { // on chip WRAM has a 32-bit bus
                let address = address - 0x3000000;
                let data = little_endian_u8_u32(self.iwram[address], self.iwram[address+1], self.iwram[address+2], self.iwram[address+3]);
                Word(data)
            }
            0x04 => {
                let address = address - 0x4000000;
                todo!();
            }
            _ => panic!("i'm not sure how to handle a bad memory read yet"),
        } 
    }
}
struct GamePak;

pub fn create_memory(file: &str) -> GeneralMemory {
    todo!();   
}