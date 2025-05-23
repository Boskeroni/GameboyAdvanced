pub mod cpu;
pub mod memory;
pub mod ppu;
pub mod joypad;

// this is so work in progress that I do not wish to track it just yet
// it won't interact with the program at all yet.
mod bus; 

use cpu::{
    handle_interrupts,
    Cpu,
    decode::{DecodedInstruction, decode_arm, decode_thumb},
    execute_arm::execute_arm,
    execute_thumb::execute_thumb,
};
use joypad::setup_joypad;
use memory::{dma_tick, update_timer, Memory};
use ppu::{tick_ppu, Ppu};

pub fn prelimenary(mem: &mut Memory) {
    setup_joypad(mem);
}

pub struct Fde {
    fetched: Option<u32>,
    decoded: Option<DecodedInstruction>,
    decoded_opcode: u32,
}
impl Fde {
    pub fn new() -> Self {
        Self {
            fetched: None,
            decoded: None,
            decoded_opcode: 0,
        }
    }
}

pub fn gba_frame(
    cpu: &mut Cpu, 
    mem: &mut Memory, 
    ppu: &mut Ppu, 
    fde: &mut Fde, 
    cycles: &mut u32,
) {
    ppu.acknowledge_frame();

    loop {
        // update the timer
        // add 1 for now, make it more accurate later
        update_timer(mem, cycles, 1);
        let active_dma = dma_tick(mem);

        tick_ppu(ppu, mem);
        if ppu.new_screen {
            ppu.new_screen = false;
            return;
        }
        if active_dma {
            continue;
        }

        let ahead_by = if fde.fetched == None { 0 } else if fde.decoded == None { 1 } else { 2 };
        handle_interrupts(mem, cpu, ahead_by);
        if cpu.clear_pipeline {
            fde.fetched = None;
            fde.decoded = None;
            cpu.clear_pipeline = false;
            continue;
        }
        // Execute
        if let Some(instruction) = fde.decoded {
            use DecodedInstruction::*;
            match instruction {
                Thumb(instr) => execute_thumb(fde.decoded_opcode as u16, instr, cpu, mem),
                Arm(instr) => execute_arm(fde.decoded_opcode, instr, cpu, mem),
            };

            if cpu.clear_pipeline {
                fde.fetched = None;
                fde.decoded = None;
                cpu.clear_pipeline = false;
                continue;
            }
        }

        // Decode
        if let Some(opcode) = fde.fetched {
            fde.decoded = Some(match cpu.cpsr.t {
                true => DecodedInstruction::Thumb(decode_thumb(opcode as u16)),
                false => DecodedInstruction::Arm(decode_arm(opcode)),
            });

            fde.decoded_opcode = opcode;
        }

        // Fetch
        fde.fetched = Some(match cpu.cpsr.t {
            true => mem.read_u16(cpu.get_pc_thumb()) as u32,
            false => mem.read_u32(cpu.get_pc_arm()),
        });
    }
}