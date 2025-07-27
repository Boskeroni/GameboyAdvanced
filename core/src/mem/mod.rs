pub mod memory;
pub mod bus;
pub mod carts;

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
pub fn split_memory_address(address: u32) -> (u32, usize) {
    ((address >> 24) & 0xF, (address & 0xFFFFFF) as usize)
}

#[inline]
fn is_in_video_memory(upp_add: u32) -> bool {
    return (upp_add == 0x5) | (upp_add == 0x6) | (upp_add == 0x7);
}
fn has_read_lock(address: u32) -> bool {
    let (hi, lo) = split_memory_address(address);
    if hi != 4 { return false; }
    match lo & !(0b1) {
        0x10..=0x46 => return true,
        0x4C        => return true,
        0x54        => return true,
        0xA0..=0xB8 => return true,
        0xBC..=0xC4 => return true,
        0xC8..=0xD0 => return true,
        0xD4..=0xDC => return true,
        0x301       => return true,
        _           => return false,
    }
}
fn has_write_lock(address: u32) -> bool {
    let (hi, lo) = split_memory_address(address);
    if hi != 4 { return false; }
    match lo & !(0b1) { // remove final bit as not important
        0x6   => return true,
        0x130 => return true,
        _     => return false,
    }
}

pub const fn get_memory_ranges() -> [std::ops::Range<u32>; 9] {
    return [
        0..0x00004000,
        0x02000000..0x02040000,
        0x03000000..0x03008000,
        0x04000000..0x040003FF,
        0x05000000..0x05000400,
        0x06000000..0x06018000,
        0x07000000..0x07000400,
        0x08000000..0x0A000000,
        0x0E000000..0x0E010000,
    ];
}