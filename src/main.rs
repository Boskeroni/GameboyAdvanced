mod cpu;
mod memory;

use cpu::registers::{CpuRegisters, status_registers::StatusRegisters};
use cpu::decode::decode_arm;
use cpu::execute_arm::execute_arm;
use memory::MemoryRead;

/// the first integer becomes the lower 8-bits
/// output =>
/// 0bBBBBBBBBAAAAAAAA
pub fn little_endian_u8_u16(a: u8, b: u8) -> u16 {
    return (b as u16) << 8 + a as u16
}
/// the first integer becomes the lower 16-bit
/// output => 
/// 0bDDDDDDDDCCCCCCCCBBBBBBBBAAAAAAAA
pub fn little_endian_u8_u32(a: u8, b: u8, c: u8, d: u8) -> u32 {
    let (a, b, c, d) = (a as u32, b as u32, c as u32, d as u32);
    return (d<<24) | (c<<16) | (b<<8) | (a);
}

fn main() {
    let mut cpu_regs = CpuRegisters {
        pc: 0,
        unbanked_registers: [0, 0, 0, 0, 0, 0, 0 ,0],
        double_banked_registers: [0, 0,  0, 0,  0, 0,  0, 0,  0, 0],
        many_banked_registers: [0, 0, 0, 0, 0, 0,  0, 0, 0, 0, 0, 0],
    };
    let mut status_registers = StatusRegisters::new();
    let memory = memory::create_memory("golden_sun.gba");
    let mut opcode = 0;
    // each loop represents one CPU cycle.
    loop {
        // fetch
        let read = memory.read(cpu_regs.get_pc());

        if status_registers.cpsr.t {
            let instruction = match read {
                // we have to do 2 memory reads
                MemoryRead::Byte(r2) => {
                    if let MemoryRead::Byte(r1) = memory.read(cpu_regs.get_pc())  {
                        ((r1 as u16) << 8) + r2 as u16
                    } else {
                        panic!("shit memory reading");
                    }  
                },
                MemoryRead::Halfword(r) => r,
                MemoryRead::Word(r) => r as u16
            };
        }

        // decode
        // we are decoding an arm isntruction

        let decoded_arm = decode_arm(opcode);

        // execute
        if status_registers.cpsr.t {
            execute_arm(opcode, decoded_arm, &mut cpu_regs, &mut status_registers)
        }
    }
}