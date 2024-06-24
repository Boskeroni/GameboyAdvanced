const BIOS: &[u8; 0x4000] = include_bytes!("../bios/gba_bios.bin");

/// the first integer becomes the lower 8-bits
/// output =>
/// 0bBBBBBBBBAAAAAAAA
fn little_endian_u8_u16(a: u8, b: u8) -> u16 {
    return (b as u16) << 8 + a as u16
}
/// the first integer becomes the lower 16-bit
/// output => 
/// 0bDDDDDDDDCCCCCCCCBBBBBBBBAAAAAAAA
fn little_endian_u8_u32(a: u8, b: u8, c: u8, d: u8) -> u32 {
    let (a, b, c, d) = (a as u32, b as u32, c as u32, d as u32);
    return (d<<24) | (c<<16) | (b<<8) | (a);
}

pub struct GeneralMemory {
    ewram: [u8; 0x40000],
    iwram: [u8; 0x8000], // 32Kbyte
    io_reg: [u8; 0x400],
    bg_pallete_ram: [u8; 0x400],
    vram: [u8; 0x18000],
    oam: [u8; 0x400],
}
impl GeneralMemory {
    pub fn read(&self, address: u32) -> u32 {
        // the upper 4 bits of the index are ignored
        let address = (address & 0x0FFFFFFF) as usize;
        
        match address >> 24 {
            0x00 => return little_endian_u8_u32(BIOS[address], BIOS[address+1], BIOS[address+2], BIOS[address+3]),
            0x02 => {}
            _ => panic!("i'm not sure how to handle a bad memory read yet"),
        }
        panic!("the memory address provided was bad")
    }
}
struct GamePak;

pub fn create_memory(file: &str) -> GeneralMemory {
    todo!();
}