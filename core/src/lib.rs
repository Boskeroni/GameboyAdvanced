pub mod cpu;
pub mod memory;
pub mod ppu;
pub mod joypad;
mod bus; 

use cpu::{
    decode::*, 
    execute_arm::execute_arm, 
    execute_thumb::execute_thumb, 
    handle_interrupts, 
    Cpu,
};
use memory::*;
use ppu::*;

#[derive(Debug, Clone, Copy)]
pub struct Fde {
    pub fetched: Option<u32>,
    pub decoded: Option<DecodedInstruction>,
    pub decoded_opcode: u32,
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
        let res = run_single_step_inner(cpu, mem, ppu, fde, cycles);
        if res {
            return;
        }
    }
}

pub fn run_single_step(
    cpu: &mut Cpu, 
    mem: &mut Memory, 
    ppu: &mut Ppu, 
    fde: &mut Fde, 
    cycles: &mut u32,   
) {
    run_single_step_inner(cpu, mem, ppu, fde, cycles);
}

fn run_single_step_inner(
    cpu: &mut Cpu, 
    mem: &mut Memory, 
    ppu: &mut Ppu, 
    fde: &mut Fde, 
    cycles: &mut u32,
) -> bool {
    // update the timer
    // add 1 for now, make it more accurate later
    update_timer(mem, cycles, 1);
    let active_dma = dma_tick(mem);

    tick_ppu(ppu, mem);
    if ppu.new_screen {
        ppu.new_screen = false;
        return true;
    }
    if active_dma {
        return false;
    }

    let ahead_by = if fde.fetched == None { 0 } else if fde.decoded == None { 1 } else { 2 };
    handle_interrupts(mem, cpu, ahead_by);
    if cpu.clear_pipeline {
        fde.fetched = None;
        fde.decoded = None;
        cpu.clear_pipeline = false;
        return false;
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
            return false;
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

    return false;
}