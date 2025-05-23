#![allow(unused)]
use crate::memory::Memory;

struct Bus {
    last_bios_fetch: u32,
    is_in_bios: bool,

    last_fetched_opcode: u32,
}

impl Bus {
    pub fn new(mem: &Memory, from_bios: bool) -> Self {
        // starting from the bios
        if from_bios {
            // these values will be instantly updated anyways
            return Self {
                last_bios_fetch: 0x0,
                is_in_bios: true,
                last_fetched_opcode: 0x0,
            }
        }

        let last_fetched = mem.read_u32(0x00DC + 8);
        Self {
            last_bios_fetch: last_fetched, // not sure why it is done like this (probably some pre-fetching thing)
            is_in_bios: false,
            last_fetched_opcode: last_fetched,
        }
    }

    pub fn fetch_arm_opcode(&mut self, pc: u32, mem: &Memory) -> u32 {
        todo!();
    }
    pub fn fetch_thumb_opcode(&mut self, pc: u32, mem: &Memory) -> u16 {
        todo!();
    }
}